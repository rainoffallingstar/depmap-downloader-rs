use crate::error::{DepMapError, Result};
use crate::models::*;
use chrono::{DateTime, Duration, Utc};
use sqlx::{Row, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tracing::{debug, error, info, warn};

pub struct CacheManager {
    db_pool: SqlitePool,
    base_url: String,
    client: reqwest::Client,
}

impl CacheManager {
    pub async fn new(database_url: &str, api_url: &str, cache_dir: &PathBuf) -> Result<Self> {
        info!("Initializing cache manager with database: {}", database_url);

        // Create cache directory if it doesn't exist
        fs::create_dir_all(cache_dir).await?;

        // Initialize database connection pool with error handling
        let db_url = format!("sqlite:{}?mode=rwc", database_url);
        info!("Attempting to connect to database: {}", db_url);

        // Try to create parent directory if needed
        if let Some(parent) = PathBuf::from(database_url).parent() {
            fs::create_dir_all(parent).await?;
        }

        let db_pool = match SqlitePool::connect(&db_url).await {
            Ok(pool) => {
                info!("Database connection successful");
                pool
            }
            Err(sqlx::Error::Database(db_err)) => {
                let message = db_err.message();
                warn!("Database connection error: {}", message);

                // Check if this is a WAL file related error AND database file exists
                let db_path = PathBuf::from(database_url);
                let db_exists = db_path.exists();

                if message.contains("unable to open database file") && db_exists {
                    warn!("Database exists but connection failed, attempting WAL recovery...");
                    Self::attempt_database_recovery(database_url).await?;
                    info!("Attempting to reconnect after recovery...");
                    SqlitePool::connect(&db_url).await?
                } else {
                    warn!(
                        "Database connection failed, trying with different connection options..."
                    );
                    // Try with explicit create mode
                    let fallback_url = format!("sqlite:{}?mode=rwc", database_url);
                    match SqlitePool::connect(&fallback_url).await {
                        Ok(pool) => {
                            info!("Database created successfully with fallback URL");
                            pool
                        }
                        Err(e) => {
                            error!("Failed to create database: {:?}", e);
                            return Err(DepMapError::DatabaseError(e));
                        }
                    }
                }
            }
            Err(e) => {
                error!("Non-database error during connection: {:?}", e);
                return Err(DepMapError::DatabaseError(e));
            }
        };

        // Configure database for better concurrent performance
        sqlx::query("PRAGMA busy_timeout = 30000") // 30 second timeout
            .execute(&db_pool)
            .await?;

        sqlx::query("PRAGMA synchronous = NORMAL") // Balance between safety and speed
            .execute(&db_pool)
            .await?;

        // Try to enable WAL mode with fallback to DELETE mode
        match sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&db_pool)
            .await
        {
            Ok(_) => info!("WAL mode enabled for better performance"),
            Err(e) => {
                warn!(
                    "Failed to enable WAL mode, falling back to DELETE mode: {}",
                    e
                );
                sqlx::query("PRAGMA journal_mode = DELETE")
                    .execute(&db_pool)
                    .await?;
            }
        }

        // Run migrations
        Self::run_migrations(&db_pool).await?;

        let client = reqwest::Client::builder()
            .user_agent("depmap-downloader-rs/0.1.0")
            .build()?;

        Ok(CacheManager {
            db_pool,
            base_url: api_url.to_string(),
            client,
        })
    }

    /// Attempt to recover database from WAL mode issues
    async fn attempt_database_recovery(database_url: &str) -> Result<()> {
        info!("Attempting database recovery...");

        // Database file existence is already checked by the caller
        let db_path = PathBuf::from(database_url);

        // Try to switch to DELETE mode using sqlite3 command
        let output = Command::new("sqlite3")
            .arg(database_url)
            .arg("PRAGMA journal_mode = DELETE;")
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    info!("Database recovery successful: switched to DELETE mode");
                    return Ok(());
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    warn!("SQLite command failed: {}", stderr);
                }
            }
            Err(e) => {
                warn!("Failed to execute sqlite3 command: {}", e);
            }
        }

        // Fallback: try to remove WAL files if they exist
        let wal_path = db_path.with_extension("db-wal");
        let shm_path = db_path.with_extension("db-shm");

        if wal_path.exists() {
            if let Err(e) = tokio::fs::remove_file(&wal_path).await {
                warn!("Failed to remove WAL file {}: {}", wal_path.display(), e);
            }
        }

        if shm_path.exists() {
            if let Err(e) = tokio::fs::remove_file(&shm_path).await {
                warn!("Failed to remove SHM file {}: {}", shm_path.display(), e);
            }
        }

        info!("Database recovery attempt completed");
        Ok(())
    }

    async fn run_migrations(db_pool: &SqlitePool) -> Result<()> {
        info!("Running database migrations...");

        // Create releases table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS releases (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                release_date TEXT,
                is_current BOOLEAN DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(db_pool)
        .await?;

        // Create files table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                url TEXT NOT NULL,
                md5_hash TEXT,
                size INTEGER,
                release_id TEXT,
                data_type TEXT,
                is_downloaded BOOLEAN DEFAULT FALSE,
                download_path TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_updated DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (release_id) REFERENCES releases (id)
            )
            "#,
        )
        .execute(db_pool)
        .await?;

        // Create datasets table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS datasets (
                id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                data_type TEXT NOT NULL,
                download_entry_url TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(db_pool)
        .await?;

        // Create dataset_files join table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS dataset_files (
                dataset_id TEXT,
                file_id INTEGER,
                PRIMARY KEY (dataset_id, file_id),
                FOREIGN KEY (dataset_id) REFERENCES datasets (id),
                FOREIGN KEY (file_id) REFERENCES files (id)
            )
            "#,
        )
        .execute(db_pool)
        .await?;

        // Create cell_lines table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cell_lines (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                lineage TEXT,
                tissue TEXT,
                datasets_available TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(db_pool)
        .await?;

        // Create gene_dependencies table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS gene_dependencies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                entrez_id INTEGER,
                gene TEXT,
                dataset TEXT,
                dependent_cell_lines REAL,
                cell_lines_with_data REAL,
                strongly_selective BOOLEAN,
                common_essential BOOLEAN,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(db_pool)
        .await?;

        // Create cache_metadata table for tracking update times
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cache_metadata (
                key TEXT PRIMARY KEY,
                value TEXT,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(db_pool)
        .await?;

        // Deduplicate historical rows before applying the unique index.
        Self::deduplicate_files_table(db_pool).await?;

        // Indexes for query speed and deduplication
        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_files_unique_release_filename_url
            ON files (COALESCE(release_id, ''), filename, url)
            "#,
        )
        .execute(db_pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_files_filename
            ON files (filename)
            "#,
        )
        .execute(db_pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_files_data_type
            ON files (data_type)
            "#,
        )
        .execute(db_pool)
        .await?;

        info!("Database migrations completed");
        Ok(())
    }

    async fn deduplicate_files_table(db_pool: &SqlitePool) -> Result<()> {
        // Keep exactly one row per logical file key and prefer rows that already
        // carry download state/path metadata.
        let delete_sql = r#"
            DELETE FROM files
            WHERE id IN (
                SELECT id FROM (
                    SELECT
                        id,
                        ROW_NUMBER() OVER (
                            PARTITION BY COALESCE(release_id, ''), filename, url
                            ORDER BY
                                CASE WHEN is_downloaded THEN 1 ELSE 0 END DESC,
                                CASE WHEN download_path IS NOT NULL AND download_path != '' THEN 1 ELSE 0 END DESC,
                                COALESCE(last_updated, '') DESC,
                                id DESC
                        ) AS rn
                    FROM files
                )
                WHERE rn > 1
            )
        "#;

        let result = sqlx::query(delete_sql).execute(db_pool).await?;
        if result.rows_affected() > 0 {
            info!(
                "Deduplicated files table: removed {} duplicate rows",
                result.rows_affected()
            );
        }
        Ok(())
    }

    pub async fn update_cache(&self, force: bool) -> Result<()> {
        info!("Updating DepMap cache...");

        if !force && !self.is_cache_expired().await? {
            info!("Cache is up to date. Use --force to update anyway.");
            return Ok(());
        }

        // Parallel fetch of all API endpoints
        match tokio::try_join!(
            self.fetch_and_cache_files(),
            self.fetch_and_cache_datasets(),
            self.fetch_and_cache_gene_dependencies()
        ) {
            Ok((_, _, _)) => {
                self.rebuild_dataset_file_links().await?;
                self.update_timestamps().await?;
                info!("Cache updated successfully!");
            }
            Err(e) => {
                warn!("Some cache updates failed: {}, but continuing...", e);
                if let Err(link_err) = self.rebuild_dataset_file_links().await {
                    warn!("Failed to rebuild dataset-file links: {}", link_err);
                }
                self.update_timestamps().await?;
            }
        }

        Ok(())
    }

    /// Update cache selectively by data types
    pub async fn update_cache_selective(
        &self,
        force: bool,
        data_type_filters: Vec<String>,
    ) -> Result<()> {
        info!(
            "Updating DepMap cache for specific data types: {:?}",
            data_type_filters
        );

        if !force && !self.is_cache_expired().await? {
            info!("Cache is up to date. Use --force to update anyway.");
            return Ok(());
        }

        // Fetch all data to get the latest information
        match tokio::try_join!(
            self.fetch_and_cache_files(),
            self.fetch_and_cache_datasets(),
            self.fetch_and_cache_gene_dependencies()
        ) {
            Ok((_, _, _)) => {
                self.rebuild_dataset_file_links().await?;
                self.update_timestamps().await?;
                info!("Cache updated successfully!");
            }
            Err(e) => {
                warn!("Some cache updates failed: {}, but continuing...", e);
                if let Err(link_err) = self.rebuild_dataset_file_links().await {
                    warn!("Failed to rebuild dataset-file links: {}", link_err);
                }
                self.update_timestamps().await?;
            }
        }

        info!("Selective update complete. All data types updated to maintain consistency.");
        Ok(())
    }

    async fn fetch_and_cache_files(&self) -> Result<()> {
        info!("Fetching files from DepMap API...");
        let url = format!("{}/download/files", self.base_url);
        let response = self.client.get(&url).send().await?;
        let csv_content = response.text().await?;

        debug!("Parsing files CSV...");
        let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());

        let mut releases: std::collections::HashMap<String, Release> =
            std::collections::HashMap::new();

        for result in rdr.deserialize() {
            let record: DownloadCsvRow = result?;

            // Extract release info
            let release_entry = releases
                .entry(record.release.clone())
                .or_insert_with(|| Release {
                    id: record.release.clone(),
                    name: record.release.clone(),
                    release_date: self.parse_date(&record.release_date),
                    files: Vec::new(),
                    is_current: false,
                    created_at: Some(Utc::now()),
                });

            // Create file entry
            let file = DownloadFile {
                id: None,
                filename: record.filename.clone(),
                url: record.url,
                md5_hash: Some(record.md5_hash),
                size: None,
                data_type: self.infer_data_type(&record.filename),
                release_id: Some(record.release.clone()),
                is_downloaded: false,
                download_path: None,
                created_at: Some(Utc::now()),
            };

            release_entry.files.push(file);
        }

        let current_release_id = releases
            .values()
            .max_by_key(|r| r.release_date)
            .map(|r| r.id.clone());

        // Store in database
        let release_count = releases.len();
        for (_, mut release) in releases {
            release.is_current = current_release_id
                .as_deref()
                .is_some_and(|current| current == release.id);
            self.store_release(&release).await?;
        }

        info!("Cached {} releases with their files", release_count);
        Ok(())
    }

    async fn fetch_latest_download_rows(&self) -> Result<Vec<DownloadCsvRow>> {
        let url = format!("{}/download/files", self.base_url);
        let response = self.client.get(&url).send().await?;
        let csv_content = response.text().await?;

        let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());
        let mut rows = Vec::new();
        for result in rdr.deserialize() {
            let record: DownloadCsvRow = result?;
            rows.push(record);
        }
        Ok(rows)
    }

    fn release_hint_matches(hint: Option<&str>, candidate_release: &str) -> bool {
        if let Some(h) = hint {
            let h_norm = h.to_ascii_lowercase();
            let c_norm = candidate_release.to_ascii_lowercase();
            return c_norm == h_norm || c_norm.contains(&h_norm) || h_norm.contains(&c_norm);
        }
        false
    }

    /// Refresh URLs/checksums from DepMap API for selected files.
    /// Falls back to cached values unless strict mode is enabled.
    pub async fn refresh_files_from_api_by_filename(
        &self,
        files: &[DownloadFile],
        strict: bool,
    ) -> Result<Vec<DownloadFile>> {
        if files.is_empty() {
            return Ok(Vec::new());
        }

        let rows = match self.fetch_latest_download_rows().await {
            Ok(rows) => rows,
            Err(e) => {
                if strict {
                    return Err(e);
                }
                warn!(
                    "Failed to refresh file links from API, fallback to cached URLs: {}",
                    e
                );
                return Ok(files.to_vec());
            }
        };

        let mut by_filename: HashMap<String, Vec<DownloadCsvRow>> = HashMap::new();
        for row in rows {
            by_filename
                .entry(row.filename.clone())
                .or_default()
                .push(row);
        }

        let mut unresolved = Vec::new();
        let mut refreshed = Vec::with_capacity(files.len());

        for file in files {
            let candidates = by_filename.get(&file.filename);
            let selected = candidates.and_then(|rows| {
                rows.iter()
                    .find(|r| Self::release_hint_matches(file.release_id.as_deref(), &r.release))
                    .cloned()
                    .or_else(|| rows.first().cloned())
            });

            if let Some(row) = selected {
                let mut updated = file.clone();
                updated.url = row.url.clone();
                updated.md5_hash = Some(row.md5_hash.clone());
                if updated.release_id.is_none() {
                    updated.release_id = Some(row.release.clone());
                }
                refreshed.push(updated);
            } else {
                unresolved.push(file.filename.clone());
                refreshed.push(file.clone());
            }
        }

        if !unresolved.is_empty() {
            if strict {
                return Err(DepMapError::DownloadError(format!(
                    "Strict refresh failed for {} files: {}",
                    unresolved.len(),
                    unresolved.join(", ")
                )));
            }
            warn!(
                "URL refresh unresolved for {} files, using cached entries",
                unresolved.len()
            );
        }

        Ok(refreshed)
    }

    async fn fetch_and_cache_datasets(&self) -> Result<()> {
        info!("Fetching datasets from DepMap API...");
        let url = format!("{}/download/datasets", self.base_url);
        let response = self.client.get(&url).send().await?;
        let datasets: Vec<DatasetApiResponse> = response.json().await?;

        debug!("Processing {} datasets...", datasets.len());

        let dataset_count = datasets.len();
        for api_dataset in datasets {
            let dataset = Dataset {
                id: api_dataset.id.clone(),
                display_name: api_dataset.display_name,
                data_type: api_dataset.data_type,
                download_entry_url: api_dataset.download_entry_url,
                associated_files: Vec::new(), // Will be populated from files table
                created_at: Some(Utc::now()),
            };

            self.store_dataset(&dataset).await?;
        }

        info!("Cached {} datasets", dataset_count);
        Ok(())
    }

    async fn fetch_and_cache_gene_dependencies(&self) -> Result<()> {
        info!("Fetching gene dependencies from DepMap API...");
        let url = format!("{}/download/gene_dep_summary", self.base_url);

        // For large files, stream the response
        let response = self.client.get(&url).send().await?;
        let bytes = response.bytes().await?;
        let mut rdr = csv::Reader::from_reader(bytes.as_ref());
        let mut processed_count = 0;

        // Check headers first to see if we have the expected format
        let headers = rdr.headers()?;
        debug!("Gene dependency CSV headers: {:?}", headers);

        // Check if required fields are present
        let has_gene_field = headers
            .iter()
            .any(|h| h.contains("gene") || h.contains("Gene"));
        let has_entrez_field = headers
            .iter()
            .any(|h| h.contains("entrez") || h.contains("Entrez"));

        if !has_gene_field || !has_entrez_field {
            warn!("Gene dependency API has unexpected format. Skipping gene dependency data.");
            warn!(
                "Expected headers with 'gene' and 'entrez' fields, got: {:?}",
                headers
            );
            return Ok(());
        }

        // Batch insert for better performance
        let mut batch: Vec<GeneDependency> = Vec::new();
        let batch_size = 500; // Process in batches of 500 records

        for result in rdr.deserialize::<GeneDependencyCsvRow>() {
            match result {
                Ok(record) => {
                    let gene_dep = GeneDependency::from(record);
                    batch.push(gene_dep);
                    processed_count += 1;

                    // Process batch when it reaches the batch size
                    if batch.len() >= batch_size {
                        self.store_gene_dependencies_batch(&batch).await?;
                        batch.clear();
                        debug!("Processed {} gene dependency records...", processed_count);
                    }
                }
                Err(e) => {
                    // Log parsing error but continue processing
                    warn!("Error parsing gene dependency record: {}, skipping...", e);
                    continue;
                }
            }
        }

        // Process remaining records in the final batch
        if !batch.is_empty() {
            self.store_gene_dependencies_batch(&batch).await?;
        }

        info!("Cached {} gene dependency records", processed_count);
        Ok(())
    }

    async fn rebuild_dataset_file_links(&self) -> Result<()> {
        info!("Rebuilding dataset-file links...");

        sqlx::query("DELETE FROM dataset_files")
            .execute(&self.db_pool)
            .await?;

        let datasets = self.get_datasets(None).await?;
        let mut linked_rows = 0usize;

        for dataset in datasets {
            let file_ids = self.match_file_ids_for_dataset(&dataset).await?;
            for file_id in file_ids {
                sqlx::query(
                    "INSERT OR IGNORE INTO dataset_files (dataset_id, file_id) VALUES (?, ?)",
                )
                .bind(&dataset.id)
                .bind(file_id)
                .execute(&self.db_pool)
                .await?;
                linked_rows += 1;
            }
        }

        info!("Rebuilt dataset-file links: {} rows", linked_rows);
        Ok(())
    }

    async fn match_file_ids_for_dataset(&self, dataset: &Dataset) -> Result<Vec<i64>> {
        let mut ids = HashSet::new();

        // First pass: parse explicit release/file from download_entry_url.
        if let Some(entry_url) = dataset.download_entry_url.as_deref() {
            let release_name = Self::extract_query_param(entry_url, "release")
                .map(|s| Self::decode_query_value(&s));
            let filename =
                Self::extract_query_param(entry_url, "file").map(|s| Self::decode_query_value(&s));

            if let Some(file_name) = filename.as_deref() {
                let exact_rows = if let Some(release) = release_name.as_deref() {
                    sqlx::query(
                        "SELECT id FROM files WHERE filename = ? AND COALESCE(release_id,'') LIKE ?",
                    )
                    .bind(file_name)
                    .bind(format!("%{}%", release))
                    .fetch_all(&self.db_pool)
                    .await?
                } else {
                    sqlx::query("SELECT id FROM files WHERE filename = ?")
                        .bind(file_name)
                        .fetch_all(&self.db_pool)
                        .await?
                };

                for row in exact_rows {
                    ids.insert(row.get::<i64, _>("id"));
                }
            }

            if ids.is_empty() {
                if let Some(release) = release_name.as_deref() {
                    let release_rows =
                        sqlx::query("SELECT id FROM files WHERE COALESCE(release_id, '') LIKE ?")
                            .bind(format!("%{}%", release))
                            .fetch_all(&self.db_pool)
                            .await?;
                    for row in release_rows {
                        ids.insert(row.get::<i64, _>("id"));
                    }
                }
            }
        }

        // Second pass: fallback fuzzy match with dataset name.
        if ids.is_empty() {
            let dataset_name_pattern = format!("%{}%", dataset.display_name);
            let strong_rows =
                sqlx::query("SELECT id FROM files WHERE filename LIKE ? OR release_id LIKE ?")
                    .bind(&dataset_name_pattern)
                    .bind(&dataset_name_pattern)
                    .fetch_all(&self.db_pool)
                    .await?;
            for row in strong_rows {
                ids.insert(row.get::<i64, _>("id"));
            }
        }

        Ok(ids.into_iter().collect())
    }

    fn extract_query_param(url: &str, key: &str) -> Option<String> {
        let query = url.split_once('?')?.1;
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == key {
                    return Some(v.to_string());
                }
            }
        }
        None
    }

    fn decode_query_value(raw: &str) -> String {
        let replaced = raw.replace('+', " ");
        let mut out = String::with_capacity(replaced.len());
        let bytes = replaced.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hex = &replaced[i + 1..i + 3];
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v as char);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i] as char);
            i += 1;
        }
        out
    }

    // Simplified get_releases without macros
    pub async fn get_releases(&self, filter: Option<&str>) -> Result<Vec<Release>> {
        let mut releases = Vec::new();

        let rows = if let Some(f) = filter {
            sqlx::query("SELECT * FROM releases WHERE name LIKE ? ORDER BY release_date DESC")
                .bind(format!("%{}%", f))
                .fetch_all(&self.db_pool)
                .await?
        } else {
            sqlx::query("SELECT * FROM releases ORDER BY release_date DESC")
                .fetch_all(&self.db_pool)
                .await?
        };

        for row in rows {
            let release_id: String = row.get("id");
            let release = Release {
                id: release_id.clone(),
                name: row.get("name"),
                release_date: row
                    .get::<Option<String>, _>("release_date")
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
                files: self.get_files_by_release_id(&release_id).await?,
                is_current: row.get("is_current"),
                created_at: row
                    .get::<Option<String>, _>("created_at")
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
            };
            releases.push(release);
        }

        Ok(releases)
    }

    pub async fn get_datasets(&self, data_type: Option<&str>) -> Result<Vec<Dataset>> {
        let mut datasets = Vec::new();

        let rows = if let Some(dt) = data_type {
            sqlx::query("SELECT * FROM datasets WHERE data_type = ? ORDER BY display_name")
                .bind(dt)
                .fetch_all(&self.db_pool)
                .await?
        } else {
            sqlx::query("SELECT * FROM datasets ORDER BY data_type, display_name")
                .fetch_all(&self.db_pool)
                .await?
        };

        for row in rows {
            let dataset = Dataset {
                id: row.get("id"),
                display_name: row.get("display_name"),
                data_type: row.get("data_type"),
                download_entry_url: row.get("download_entry_url"),
                associated_files: Vec::new(), // Would need to populate separately
                created_at: row
                    .get::<Option<String>, _>("created_at")
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
            };
            datasets.push(dataset);
        }

        Ok(datasets)
    }

    // Stub implementations for other methods
    pub async fn search_cell_lines(&self, _query: &str) -> Result<Vec<CellLine>> {
        Ok(Vec::new())
    }

    pub async fn search_datasets(&self, _query: &str) -> Result<Vec<Dataset>> {
        Ok(Vec::new())
    }

    pub async fn search_genes(&self, query: &str, limit: usize) -> Result<Vec<GeneDependency>> {
        let pattern = format!("%{}%", query);

        let rows = sqlx::query(
            "SELECT * FROM gene_dependencies 
             WHERE gene LIKE ? OR entrez_id = ?
             ORDER BY 
                 CASE 
                     WHEN gene LIKE ? THEN 1
                     WHEN gene LIKE ? THEN 2
                     ELSE 3
                 END,
                 dependent_cell_lines DESC
             LIMIT ?",
        )
        .bind(&pattern)
        .bind(query) // Try to match exact Entrez ID if query is numeric
        .bind(format!("{}%", query)) // Starts with query
        .bind(format!("%{}", query)) // Ends with query
        .bind(limit as i64)
        .fetch_all(&self.db_pool)
        .await?;

        let mut genes = Vec::new();
        for row in rows {
            genes.push(GeneDependency {
                id: Some(row.get("id")),
                entrez_id: row.get("entrez_id"),
                gene: row.get("gene"),
                dataset: row.get("dataset"),
                dependent_cell_lines: row.get("dependent_cell_lines"),
                cell_lines_with_data: row.get("cell_lines_with_data"),
                strongly_selective: row.get("strongly_selective"),
                common_essential: row.get("common_essential"),
                created_at: row
                    .get::<Option<String>, _>("created_at")
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
            });
        }

        Ok(genes)
    }

    pub async fn get_file_by_name(&self, filename: &str) -> Result<DownloadFile> {
        let row = sqlx::query("SELECT * FROM files WHERE filename = ?")
            .bind(filename)
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or_else(|| DepMapError::FileNotFound(filename.to_string()))?;

        Ok(DownloadFile {
            id: Some(row.get("id")),
            filename: row.get("filename"),
            url: row.get("url"),
            md5_hash: row.get("md5_hash"),
            size: row.get::<Option<i64>, _>("size").map(|s| s as u64),
            data_type: row.get("data_type"),
            release_id: row.get("release_id"),
            is_downloaded: row.get("is_downloaded"),
            download_path: row.get("download_path"),
            created_at: row
                .get::<Option<String>, _>("created_at")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
        })
    }

    pub async fn get_dataset_files(&self, dataset_id: &str) -> Result<Vec<DownloadFile>> {
        let dataset = self.resolve_dataset(dataset_id).await?;

        // First try via dataset_files join table.
        let rows = sqlx::query(
            "SELECT f.* FROM dataset_files df JOIN files f ON df.file_id = f.id WHERE df.dataset_id = ?"
        )
        .bind(&dataset.id)
        .fetch_all(&self.db_pool)
        .await?;

        if !rows.is_empty() {
            let mut files: Vec<DownloadFile> = rows
                .iter()
                .map(|row| self.row_to_download_file(row))
                .collect();
            files.sort_by(|a, b| a.filename.cmp(&b.filename));
            files.dedup_by(|a, b| a.filename == b.filename && a.url == b.url);
            return Ok(files);
        }

        // If join table is empty, run fallback matching and return matched files.
        let file_ids = self.match_file_ids_for_dataset(&dataset).await?;
        if file_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        for file_id in file_ids {
            if let Some(row) = sqlx::query("SELECT * FROM files WHERE id = ?")
                .bind(file_id)
                .fetch_optional(&self.db_pool)
                .await?
            {
                files.push(self.row_to_download_file(&row));
            }
        }

        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        files.dedup_by(|a, b| a.filename == b.filename && a.url == b.url);
        Ok(files)
    }

    async fn resolve_dataset(&self, dataset_ref: &str) -> Result<Dataset> {
        let datasets = self.get_datasets(None).await?;
        datasets
            .into_iter()
            .find(|d| d.id == dataset_ref || d.display_name == dataset_ref)
            .ok_or_else(|| DepMapError::NotFound(format!("Dataset '{}' not found", dataset_ref)))
    }

    pub async fn get_current_release_core_files(&self) -> Result<Vec<DownloadFile>> {
        let releases = self.get_releases(None).await?;

        // 策略1: 查找标记为当前的release
        let current = releases.iter().find(|r| r.is_current).or_else(|| {
            // 策略2: 查找最新的release
            releases.iter().max_by_key(|r| {
                r.release_date.unwrap_or_else(|| {
                    DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&Utc)
                })
            })
        });

        match current {
            Some(release) => {
                // 选择核心数据类型的文件
                let core_files: Vec<DownloadFile> = release
                    .files
                    .clone()
                    .into_iter()
                    .filter(|f| {
                        matches!(
                            f.data_type.as_deref(),
                            Some("CRISPR")
                                | Some("Expression")
                                | Some("Mutations")
                                | Some("CN")
                                | Some("RNAi")
                                | Some("Drug screen")
                        )
                    })
                    .collect();

                Ok(core_files)
            }
            None => Err(DepMapError::NotFound(
                "No current release found".to_string(),
            )),
        }
    }

    // Private method for internal use - gets files by release ID without recursion
    async fn get_files_by_release_id(&self, release_id: &str) -> Result<Vec<DownloadFile>> {
        let mut files = Vec::new();

        let rows = sqlx::query("SELECT * FROM files WHERE release_id = ?")
            .bind(release_id)
            .fetch_all(&self.db_pool)
            .await?;

        for row in rows {
            let file = DownloadFile {
                id: Some(row.get("id")),
                filename: row.get("filename"),
                url: row.get("url"),
                md5_hash: row.get("md5_hash"),
                size: row.get::<Option<i64>, _>("size").map(|s| s as u64),
                data_type: row.get("data_type"),
                release_id: row.get("release_id"),
                is_downloaded: row.get("is_downloaded"),
                download_path: row.get("download_path"),
                created_at: row
                    .get::<Option<String>, _>("created_at")
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
            };
            files.push(file);
        }

        Ok(files)
    }

    pub async fn get_release_files(
        &self,
        release_name: &str,
        data_type_filter: Option<&str>,
    ) -> Result<Vec<DownloadFile>> {
        let releases = self.get_releases(Some(release_name)).await?;

        if releases.is_empty() {
            return Err(DepMapError::NotFound(format!(
                "Release '{}' not found",
                release_name
            )));
        }

        // Collect files from all matching releases and filter by data type if requested
        let all_files: Vec<DownloadFile> = releases
            .into_iter()
            .flat_map(|r| r.files)
            .filter(|f| {
                if let Some(data_type) = data_type_filter {
                    f.data_type
                        .as_ref()
                        .map(|dt| dt.to_lowercase().contains(&data_type.to_lowercase()))
                        .unwrap_or(false)
                } else {
                    true // No filter, include all files
                }
            })
            .collect();

        Ok(all_files)
    }

    // Helper methods
    async fn store_release(&self, release: &Release) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO releases (id, name, release_date, is_current, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(&release.id)
            .bind(&release.name)
            .bind(release.release_date.as_ref().map(|d| d.to_rfc3339()))
            .bind(release.is_current)
            .bind(release.created_at.as_ref().map(|d| d.to_rfc3339()))
            .execute(&self.db_pool)
            .await?;

        // Store files
        for file in &release.files {
            self.store_file(file).await?;
        }

        Ok(())
    }

    /// Convert database row to DownloadFile
    fn row_to_download_file(&self, row: &sqlx::sqlite::SqliteRow) -> DownloadFile {
        DownloadFile {
            id: Some(row.get("id")),
            filename: row.get("filename"),
            url: row.get("url"),
            md5_hash: row.get("md5_hash"),
            size: row.get::<Option<i64>, _>("size").map(|s| s as u64),
            data_type: row.get("data_type"),
            release_id: row.get("release_id"),
            is_downloaded: row.get("is_downloaded"),
            download_path: row.get("download_path"),
            created_at: row
                .get::<Option<String>, _>("created_at")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
        }
    }

    async fn store_dataset(&self, dataset: &Dataset) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO datasets (id, display_name, data_type, download_entry_url, created_at, last_updated) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&dataset.id)
            .bind(&dataset.display_name)
            .bind(&dataset.data_type)
            .bind(&dataset.download_entry_url)
            .bind(dataset.created_at.as_ref().map(|d| d.to_rfc3339()))
            .bind(dataset.created_at.as_ref().map(|d| d.to_rfc3339())) // Use created_at as last_updated
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }

    async fn store_file(&self, file: &DownloadFile) -> Result<()> {
        // Keep one logical row per (release_id, filename, url) and preserve download state.
        let existing = sqlx::query(
            "SELECT id, is_downloaded, download_path FROM files 
             WHERE filename = ? AND url = ? AND COALESCE(release_id, '') = COALESCE(?, '')
             LIMIT 1",
        )
        .bind(&file.filename)
        .bind(&file.url)
        .bind(&file.release_id)
        .fetch_optional(&self.db_pool)
        .await?;

        if let Some(row) = existing {
            let existing_id: i64 = row.get("id");
            let existing_downloaded: bool = row.get("is_downloaded");
            let existing_path: Option<String> = row.get("download_path");
            let merged_downloaded = existing_downloaded || file.is_downloaded;
            let merged_path = existing_path.or_else(|| file.download_path.clone());

            sqlx::query(
                "UPDATE files SET md5_hash = ?, size = ?, data_type = ?, is_downloaded = ?, download_path = ?, last_updated = ? WHERE id = ?"
            )
            .bind(&file.md5_hash)
            .bind(file.size.map(|s| s as i64))
            .bind(&file.data_type)
            .bind(merged_downloaded)
            .bind(&merged_path)
            .bind(Utc::now().to_rfc3339())
            .bind(existing_id)
            .execute(&self.db_pool)
            .await?;
            return Ok(());
        }

        sqlx::query("INSERT INTO files (filename, url, md5_hash, size, release_id, data_type, is_downloaded, download_path, created_at, last_updated) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&file.filename)
            .bind(&file.url)
            .bind(&file.md5_hash)
            .bind(file.size.map(|s| s as i64))
            .bind(&file.release_id)
            .bind(&file.data_type)
            .bind(file.is_downloaded)
            .bind(&file.download_path)
            .bind(file.created_at.as_ref().map(|d| d.to_rfc3339()))
            .bind(Utc::now().to_rfc3339())
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }

    async fn store_gene_dependencies_batch(&self, gene_deps: &[GeneDependency]) -> Result<()> {
        if gene_deps.is_empty() {
            return Ok(());
        }

        // Begin transaction
        let mut tx = self.db_pool.begin().await?;

        // Pre-convert all created_at timestamps to avoid lifetime issues
        let mut params = Vec::new();
        for gene_dep in gene_deps {
            let created_at_str = gene_dep.created_at.map(|d| d.to_rfc3339());
            params.push((
                gene_dep.entrez_id as i64,
                gene_dep.gene.clone(),
                gene_dep.dataset.clone(),
                gene_dep.dependent_cell_lines,
                gene_dep.cell_lines_with_data,
                gene_dep.strongly_selective,
                gene_dep.common_essential,
                created_at_str,
            ));
        }

        // Use the UNALL logging statement for faster bulk operations
        for (
            entrez_id,
            gene,
            dataset,
            dependent_cell_lines,
            cell_lines_with_data,
            strongly_selective,
            common_essential,
            created_at_str,
        ) in params
        {
            sqlx::query("INSERT OR REPLACE INTO gene_dependencies (entrez_id, gene, dataset, dependent_cell_lines, cell_lines_with_data, strongly_selective, common_essential, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(entrez_id)
                .bind(&gene)
                .bind(&dataset)
                .bind(dependent_cell_lines)
                .bind(cell_lines_with_data)
                .bind(strongly_selective)
                .bind(common_essential)
                .bind(created_at_str)
                .execute(&mut *tx)
                .await?;
        }

        // Commit transaction
        tx.commit().await?;

        Ok(())
    }

    async fn is_cache_expired(&self) -> Result<bool> {
        let result =
            sqlx::query("SELECT updated_at FROM cache_metadata WHERE key = 'last_full_update'")
                .fetch_optional(&self.db_pool)
                .await?;

        if let Some(row) = result {
            if let Some(updated_str) = row.get::<Option<String>, _>("updated_at") {
                let updated: DateTime<Utc> = DateTime::parse_from_rfc3339(&updated_str)?.into();
                let expired = Utc::now() - updated > Duration::hours(24);
                Ok(expired)
            } else {
                Ok(true)
            }
        } else {
            Ok(true)
        }
    }

    async fn update_timestamps(&self) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO cache_metadata (key, value, updated_at) VALUES ('last_full_update', 'completed', ?)")
            .bind(Utc::now().to_rfc3339())
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }

    fn parse_date(&self, date_str: &str) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(date_str)
            .map(|dt| dt.into())
            .ok()
    }

    fn infer_data_type(&self, filename: &str) -> Option<String> {
        let filename_lower = filename.to_lowercase();

        if filename_lower.contains("crispr") {
            Some("CRISPR".to_string())
        } else if filename_lower.contains("rnai") {
            Some("RNAi".to_string())
        } else if filename_lower.contains("expression") {
            Some("Expression".to_string())
        } else if filename_lower.contains("mutation") {
            Some("Mutations".to_string())
        } else if filename_lower.contains("copy") && filename_lower.contains("number") {
            Some("CN".to_string())
        } else if filename_lower.contains("drug") || filename_lower.contains("prism") {
            Some("Drug screen".to_string())
        } else if filename_lower.contains("protein") || filename_lower.contains("rppa") {
            Some("Protein Expression".to_string())
        } else if filename_lower.contains("metabol") {
            Some("Metabolomics".to_string())
        } else if filename_lower.contains("subtype") || filename_lower.contains("model") {
            Some("Metadata".to_string())
        } else {
            None
        }
    }

    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        let release_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM releases")
            .fetch_one(&self.db_pool)
            .await?;

        let file_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
            .fetch_one(&self.db_pool)
            .await?;

        let dataset_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM datasets")
            .fetch_one(&self.db_pool)
            .await?;

        let cell_line_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cell_lines")
            .fetch_one(&self.db_pool)
            .await?;

        let gene_dependency_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM gene_dependencies")
                .fetch_one(&self.db_pool)
                .await?;

        let total_size_mb: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM files")
            .fetch_one(&self.db_pool)
            .await?;

        let last_updated =
            sqlx::query("SELECT updated_at FROM cache_metadata WHERE key = 'last_full_update'")
                .fetch_optional(&self.db_pool)
                .await?
                .and_then(|row| row.get::<Option<String>, _>("updated_at"))
                .and_then(|dt| DateTime::parse_from_rfc3339(&dt).ok())
                .map(|dt| dt.into());

        Ok(CacheStats {
            dataset_count: dataset_count as usize,
            file_count: file_count as usize,
            release_count: release_count as usize,
            cell_line_count: cell_line_count as usize,
            gene_dependency_count: gene_dependency_count as usize,
            total_size_mb: (total_size_mb / (1024 * 1024)) as u64,
            last_updated,
        })
    }

    pub async fn mark_files_as_downloaded(
        &self,
        downloaded_files: &[(String, String)],
    ) -> Result<usize> {
        let mut updated_rows = 0usize;
        for (filename, download_path) in downloaded_files {
            let result = sqlx::query(
                "UPDATE files
                 SET is_downloaded = TRUE, download_path = ?, last_updated = ?
                 WHERE filename = ?",
            )
            .bind(download_path)
            .bind(Utc::now().to_rfc3339())
            .bind(filename)
            .execute(&self.db_pool)
            .await?;
            updated_rows += result.rows_affected() as usize;
        }
        Ok(updated_rows)
    }

    pub async fn get_download_progress(&self, output_dir: &Path) -> Result<DownloadProgressReport> {
        let mut local_files = HashSet::new();
        if output_dir.exists() {
            let mut entries = fs::read_dir(output_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        local_files.insert(name.to_string());
                    }
                }
            }
        }

        let db_marked_downloaded_files: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT filename) FROM files WHERE is_downloaded = TRUE",
        )
        .fetch_one(&self.db_pool)
        .await?;

        let by_data_rows =
            sqlx::query("SELECT DISTINCT COALESCE(data_type, ''), filename FROM files")
                .fetch_all(&self.db_pool)
                .await?;
        let by_data_type = self.build_progress_breakdown(by_data_rows, &local_files, true);

        let by_release_rows = sqlx::query(
            "SELECT DISTINCT release_id, filename FROM files WHERE COALESCE(release_id, '') != ''",
        )
        .fetch_all(&self.db_pool)
        .await?;
        let by_release = self.build_progress_breakdown(by_release_rows, &local_files, false);

        let total_files = by_data_type.iter().map(|b| b.total_files).sum::<usize>();
        let present_files = by_data_type.iter().map(|b| b.present_files).sum::<usize>();

        Ok(DownloadProgressReport {
            present_files,
            total_files,
            db_marked_downloaded_files: db_marked_downloaded_files as usize,
            by_data_type,
            by_release,
        })
    }

    fn build_progress_breakdown(
        &self,
        rows: Vec<sqlx::sqlite::SqliteRow>,
        local_files: &HashSet<String>,
        normalize_empty_as_unknown: bool,
    ) -> Vec<ProgressBreakdown> {
        let mut totals: HashMap<String, usize> = HashMap::new();
        let mut present: HashMap<String, usize> = HashMap::new();

        for row in rows {
            let mut category: String = row.get::<String, _>(0);
            let filename: String = row.get::<String, _>(1);
            if normalize_empty_as_unknown && category.trim().is_empty() {
                category = "Unknown".to_string();
            } else if category.trim().is_empty() {
                category = "(none)".to_string();
            }

            *totals.entry(category.clone()).or_insert(0) += 1;
            if local_files.contains(&filename) {
                *present.entry(category).or_insert(0) += 1;
            }
        }

        let mut breakdown: Vec<ProgressBreakdown> = totals
            .into_iter()
            .map(|(category, total_files)| ProgressBreakdown {
                present_files: *present.get(&category).unwrap_or(&0),
                total_files,
                category,
            })
            .collect();
        breakdown.sort_by(|a, b| a.category.cmp(&b.category));
        breakdown
    }

    /// Clear all cached data
    pub async fn clear_all_cache(&self) -> Result<()> {
        info!("Clearing all cached data");

        // Get all downloaded files to delete from disk
        let downloaded_files: Vec<String> = sqlx::query_scalar("SELECT download_path FROM files WHERE is_downloaded = TRUE AND download_path IS NOT NULL")
            .fetch_all(&self.db_pool)
            .await?;

        // Delete files from disk
        for file_path in downloaded_files {
            if let Err(e) = fs::remove_file(&file_path).await {
                warn!("Failed to delete file {}: {}", file_path, e);
            }
        }

        // Clear all tables
        sqlx::query("DELETE FROM gene_dependencies")
            .execute(&self.db_pool)
            .await?;
        sqlx::query("DELETE FROM cell_lines")
            .execute(&self.db_pool)
            .await?;
        sqlx::query("DELETE FROM files")
            .execute(&self.db_pool)
            .await?;
        sqlx::query("DELETE FROM datasets")
            .execute(&self.db_pool)
            .await?;
        sqlx::query("DELETE FROM releases")
            .execute(&self.db_pool)
            .await?;
        sqlx::query("DELETE FROM cache_metadata")
            .execute(&self.db_pool)
            .await?;

        info!("All cached data cleared successfully");
        Ok(())
    }

    /// Clear cached data of a specific type
    pub async fn clear_cache_by_data_type(&self, data_type: &str) -> Result<usize> {
        info!("Clearing cached data of type: {}", data_type);

        let data_type_pattern = format!("%{}%", data_type);

        // Get files to delete from disk
        let downloaded_files: Vec<String> = sqlx::query_scalar(
            "SELECT download_path FROM files WHERE data_type LIKE ? AND is_downloaded = TRUE AND download_path IS NOT NULL"
        )
        .bind(&data_type_pattern)
        .fetch_all(&self.db_pool)
        .await?;

        // Delete files from disk
        for file_path in downloaded_files {
            if let Err(e) = fs::remove_file(&file_path).await {
                warn!("Failed to delete file {}: {}", file_path, e);
            }
        }

        // Delete related records
        let deleted_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE data_type LIKE ?")
                .bind(&data_type_pattern)
                .fetch_one(&self.db_pool)
                .await?;

        sqlx::query("DELETE FROM files WHERE data_type LIKE ?")
            .bind(&data_type_pattern)
            .execute(&self.db_pool)
            .await?;

        // Check for any datasets matching this data type and remove them
        let deleted_datasets: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM datasets WHERE data_type LIKE ?")
                .bind(&data_type_pattern)
                .fetch_one(&self.db_pool)
                .await?;

        sqlx::query("DELETE FROM datasets WHERE data_type LIKE ?")
            .bind(&data_type_pattern)
            .execute(&self.db_pool)
            .await?;

        info!(
            "Cleared {} files and {} datasets of type {}",
            deleted_count, deleted_datasets, data_type
        );

        Ok(deleted_count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::CacheManager;

    #[test]
    fn matches_release_hints_case_insensitive_and_substring() {
        assert!(!CacheManager::release_hint_matches(
            None,
            "DepMap Public 25Q4"
        ));
        assert!(CacheManager::release_hint_matches(
            Some("depmap public 25q4"),
            "DepMap Public 25Q4"
        ));
        assert!(CacheManager::release_hint_matches(
            Some("25q4"),
            "DepMap Public 25Q4"
        ));
        assert!(CacheManager::release_hint_matches(
            Some("DepMap Public 25Q4"),
            "25Q4"
        ));
        assert!(!CacheManager::release_hint_matches(
            Some("25Q3"),
            "DepMap Public 25Q4"
        ));
    }

    #[test]
    fn parses_and_decodes_query_params() {
        let url = "https://example.com/download?release=DepMap+Public+25Q4&file=CCLE%5FRRBS%5FTSS1kb%5F20240101.csv&x=1";
        let release = CacheManager::extract_query_param(url, "release")
            .map(|v| CacheManager::decode_query_value(&v));
        let file = CacheManager::extract_query_param(url, "file")
            .map(|v| CacheManager::decode_query_value(&v));

        assert_eq!(release.as_deref(), Some("DepMap Public 25Q4"));
        assert_eq!(file.as_deref(), Some("CCLE_RRBS_TSS1kb_20240101.csv"));
        assert_eq!(CacheManager::extract_query_param(url, "missing"), None);
    }
}
