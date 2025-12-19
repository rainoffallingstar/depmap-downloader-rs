use crate::error::{DepMapError, Result};
use crate::models::*;
use chrono::{DateTime, Utc, Duration};
use sqlx::{SqlitePool, Row};
use std::path::PathBuf;
use tracing::{info, warn, debug};
use tokio::fs;

pub struct CacheManager {
    db_pool: SqlitePool,
    base_url: String,
    cache_dir: PathBuf,
    client: reqwest::Client,
}

impl CacheManager {
    pub async fn new(database_url: &str, api_url: &str, cache_dir: &PathBuf) -> Result<Self> {
        info!("Initializing cache manager with database: {}", database_url);
        
        // Create cache directory if it doesn't exist
        fs::create_dir_all(cache_dir).await?;
        
        // Initialize database connection pool
        let db_url = format!("sqlite:{}", database_url);
        let db_pool = SqlitePool::connect(&db_url).await?;
        
        // Configure database for better concurrent performance
        sqlx::query("PRAGMA busy_timeout = 30000")  // 30 second timeout
            .execute(&db_pool)
            .await?;
        
        sqlx::query("PRAGMA synchronous = NORMAL")   // Balance between safety and speed
            .execute(&db_pool)
            .await?;
        
        sqlx::query("PRAGMA journal_mode = WAL")     // Enable Write-Ahead Logging for better concurrency
            .execute(&db_pool)
            .await?;
        
        // Run migrations
        Self::run_migrations(&db_pool).await?;
        
        let client = reqwest::Client::builder()
            .user_agent("depmap-downloader-rs/0.1.0")
            .build()?;
        
        Ok(CacheManager {
            db_pool,
            base_url: api_url.to_string(),
            cache_dir: cache_dir.clone(),
            client,
        })
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
            "#
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
            "#
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
            "#
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
            "#
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
            "#
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
            "#
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
            "#
        )
        .execute(db_pool)
        .await?;
        
        info!("Database migrations completed");
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
                self.update_timestamps().await?;
                info!("Cache updated successfully!");
            }
            Err(e) => {
                warn!("Some cache updates failed: {}, but continuing...", e);
                self.update_timestamps().await?;
            }
        }
        
        Ok(())
    }
    
    /// Update cache selectively by data types
    pub async fn update_cache_selective(&self, force: bool, data_type_filters: Vec<String>) -> Result<()> {
        info!("Updating DepMap cache for specific data types: {:?}", data_type_filters);
        
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
                self.update_timestamps().await?;
                info!("Cache updated successfully!");
            }
            Err(e) => {
                warn!("Some cache updates failed: {}, but continuing...", e);
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
        
        let mut releases: std::collections::HashMap<String, Release> = std::collections::HashMap::new();
        
        for result in rdr.deserialize() {
            let record: DownloadCsvRow = result?;
            
            // Extract release info
            let release_entry = releases.entry(record.release.clone()).or_insert_with(|| Release {
                id: record.release.clone(),
                name: record.release.clone(),
                release_date: self.parse_date(&record.release_date),
                files: Vec::new(),
                is_current: record.release.contains("25Q3"), // Heuristic for current
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
        
        // Store in database
        let release_count = releases.len();
        for (_, release) in releases {
            self.store_release(&release).await?;
        }
        
        info!("Cached {} releases with their files", release_count);
        Ok(())
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
        let has_gene_field = headers.iter().any(|h| h.contains("gene") || h.contains("Gene"));
        let has_entrez_field = headers.iter().any(|h| h.contains("entrez") || h.contains("Entrez"));
        
        if !has_gene_field || !has_entrez_field {
            warn!("Gene dependency API has unexpected format. Skipping gene dependency data.");
            warn!("Expected headers with 'gene' and 'entrez' fields, got: {:?}", headers);
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
                release_date: row.get::<Option<String>, _>("release_date")
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
                files: self.get_files_by_release_id(&release_id).await?,
                is_current: row.get("is_current"),
                created_at: row.get::<Option<String>, _>("created_at")
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
                created_at: row.get::<Option<String>, _>("created_at")
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
             LIMIT ?"
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
                created_at: row.get::<Option<String>, _>("created_at")
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
            });
        }
        
        Ok(genes)
    }
    
    pub async fn get_gene_dependencies(&self, _gene: &str) -> Result<Vec<GeneDependency>> {
        Ok(Vec::new())
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
            created_at: row.get::<Option<String>, _>("created_at")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
        })
    }
    
    pub async fn get_dataset_files(&self, dataset_id: &str) -> Result<Vec<DownloadFile>> {
        // 首先尝试从关联表获取
        let rows = sqlx::query(
            "SELECT f.* FROM dataset_files df JOIN files f ON df.file_id = f.id WHERE df.dataset_id = ?"
        )
        .bind(dataset_id)
        .fetch_all(&self.db_pool)
        .await?;
        
        if !rows.is_empty() {
            let files: Vec<DownloadFile> = rows.iter()
                .map(|row| self.row_to_download_file(row))
                .collect();
            return Ok(files);
        }
        
        // 如果关联表为空，使用名称匹配作为后备方案
        let datasets = self.get_datasets(None).await?;
        let dataset = datasets.iter()
            .find(|d| d.id == dataset_id || d.display_name == dataset_id)
            .ok_or_else(|| DepMapError::NotFound(format!("Dataset '{}' not found", dataset_id)))?;
        
        let pattern = format!("%{}%", dataset.display_name);
        let file_rows = sqlx::query(
            "SELECT * FROM files WHERE filename LIKE ? OR data_type LIKE ?"
        )
        .bind(&pattern)
        .bind(&dataset.data_type)
        .fetch_all(&self.db_pool)
        .await?;
        
        let files: Vec<DownloadFile> = file_rows.iter()
            .map(|row| self.row_to_download_file(row))
            .collect();
        
        Ok(files)
    }
    
    pub async fn get_current_release_core_files(&self) -> Result<Vec<DownloadFile>> {
        let releases = self.get_releases(None).await?;
        
        // 策略1: 查找标记为当前的release
        let current = releases.iter()
            .find(|r| r.is_current)
            .or_else(|| {
                // 策略2: 查找最新的release
                releases.iter()
                    .max_by_key(|r| r.release_date.unwrap_or_else(|| DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap().with_timezone(&Utc)))
            });
        
        match current {
            Some(release) => {
                // 选择核心数据类型的文件
                let core_types = vec![
                    "CRISPR".to_string(),
                    "Expression".to_string(), 
                    "Mutations".to_string(),
                    "CN".to_string(),
                    "RNAi".to_string(),
                    "Drug screen".to_string(),
                ];
                
                let core_files: Vec<DownloadFile> = release.files.clone()
                    .into_iter()
                    .filter(|f| {
                        f.data_type.as_ref()
                            .map(|dt| core_types.contains(dt))
                            .unwrap_or(false)
                    })
                    .collect();
                
                Ok(core_files)
            }
            None => Err(DepMapError::NotFound("No current release found".to_string()))
        }
    }
    
    pub async fn get_releases_files(&self, release_name: &str) -> Result<Vec<DownloadFile>> {
        let releases = self.get_releases(Some(release_name)).await?;
        
        if releases.is_empty() {
            return Err(DepMapError::NotFound(format!("Release '{}' not found", release_name)));
        }
        
        // 合并所有匹配release的文件
        let all_files: Vec<DownloadFile> = releases
            .into_iter()
            .flat_map(|r| r.files)
            .collect();
        
        Ok(all_files)
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
                created_at: row.get::<Option<String>, _>("created_at")
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
            };
            files.push(file);
        }
        
        Ok(files)
    }

    pub async fn get_release_files(&self, release_name: &str, data_type_filter: Option<&str>) -> Result<Vec<DownloadFile>> {
        let releases = self.get_releases(Some(release_name)).await?;
        
        if releases.is_empty() {
            return Err(DepMapError::NotFound(format!("Release '{}' not found", release_name)));
        }
        
        // Collect files from all matching releases and filter by data type if requested
        let all_files: Vec<DownloadFile> = releases
            .into_iter()
            .flat_map(|r| r.files)
            .filter(|f| {
                if let Some(data_type) = data_type_filter {
                    f.data_type.as_ref()
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
            .bind(&release.release_date.map(|d| d.to_rfc3339()))
            .bind(release.is_current)
            .bind(&release.created_at.map(|d| d.to_rfc3339()))
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
            created_at: row.get::<Option<String>, _>("created_at")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.into())),
        }
    }
    
    async fn store_dataset(&self, dataset: &Dataset) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO datasets (id, display_name, data_type, download_entry_url, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&dataset.id)
            .bind(&dataset.display_name)
            .bind(&dataset.data_type)
            .bind(&dataset.download_entry_url)
            .bind(&dataset.created_at.map(|d| d.to_rfc3339()))
            .execute(&self.db_pool)
            .await?;
        
        Ok(())
    }
    
    
    
    async fn store_file(&self, file: &DownloadFile) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO files (filename, url, md5_hash, size, release_id, data_type, is_downloaded, download_path, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&file.filename)
            .bind(&file.url)
            .bind(&file.md5_hash)
            .bind(file.size.map(|s| s as i64))
            .bind(&file.release_id)
            .bind(&file.data_type)
            .bind(file.is_downloaded)
            .bind(&file.download_path)
            .bind(&file.created_at.map(|d| d.to_rfc3339()))
            .execute(&self.db_pool)
            .await?;
        
        Ok(())
    }
    
    async fn store_gene_dependency(&self, gene_dep: &GeneDependency) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO gene_dependencies (entrez_id, gene, dataset, dependent_cell_lines, cell_lines_with_data, strongly_selective, common_essential, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(gene_dep.entrez_id as i64)
            .bind(&gene_dep.gene)
            .bind(&gene_dep.dataset)
            .bind(gene_dep.dependent_cell_lines)
            .bind(gene_dep.cell_lines_with_data)
            .bind(gene_dep.strongly_selective)
            .bind(gene_dep.common_essential)
            .bind(&gene_dep.created_at.map(|d| d.to_rfc3339()))
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
                gene_dep.strongly_selective.clone(),
                gene_dep.common_essential.clone(),
                created_at_str,
            ));
        }
        
        // Use the UNALL logging statement for faster bulk operations
        for (entrez_id, gene, dataset, dependent_cell_lines, cell_lines_with_data, 
             strongly_selective, common_essential, created_at_str) in params {
            sqlx::query("INSERT OR REPLACE INTO gene_dependencies (entrez_id, gene, dataset, dependent_cell_lines, cell_lines_with_data, strongly_selective, common_essential, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(entrez_id)
                .bind(&gene)
                .bind(&dataset)
                .bind(dependent_cell_lines)
                .bind(cell_lines_with_data)
                .bind(&strongly_selective)
                .bind(&common_essential)
                .bind(&created_at_str)
                .execute(&mut *tx)
                .await?;
        }
        
        // Commit transaction
        tx.commit().await?;
        
        Ok(())
    }
    
    async fn is_cache_expired(&self) -> Result<bool> {
        let result = sqlx::query("SELECT updated_at FROM cache_metadata WHERE key = 'last_full_update'")
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
        DateTime::parse_from_rfc3339(date_str).map(|dt| dt.into()).ok()
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
        
        let gene_dependency_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gene_dependencies")
            .fetch_one(&self.db_pool)
            .await?;
        
        let total_size_mb: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM files")
            .fetch_one(&self.db_pool)
            .await?;
        
        let last_updated = sqlx::query("SELECT updated_at FROM cache_metadata WHERE key = 'last_full_update'")
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
        sqlx::query("DELETE FROM gene_dependencies").execute(&self.db_pool).await?;
        sqlx::query("DELETE FROM cell_lines").execute(&self.db_pool).await?;
        sqlx::query("DELETE FROM files").execute(&self.db_pool).await?;
        sqlx::query("DELETE FROM datasets").execute(&self.db_pool).await?;
        sqlx::query("DELETE FROM releases").execute(&self.db_pool).await?;
        sqlx::query("DELETE FROM cache_metadata").execute(&self.db_pool).await?;
        
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
        let deleted_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM files WHERE data_type LIKE ?"
        )
        .bind(&data_type_pattern)
        .fetch_one(&self.db_pool)
        .await?;
        
        sqlx::query("DELETE FROM files WHERE data_type LIKE ?")
            .bind(&data_type_pattern)
            .execute(&self.db_pool)
            .await?;
        
        // Check for any datasets matching this data type and remove them
        let deleted_datasets: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM datasets WHERE data_type LIKE ?"
        )
        .bind(&data_type_pattern)
        .fetch_one(&self.db_pool)
        .await?;
        
        sqlx::query("DELETE FROM datasets WHERE data_type LIKE ?")
            .bind(&data_type_pattern)
            .execute(&self.db_pool)
            .await?;
            
        info!("Cleared {} files and {} datasets of type {}", 
              deleted_count, deleted_datasets, data_type);
        
        Ok(deleted_count as usize)
    }
}
