use crate::error::{DepMapError, Result};
use crate::models::*;
use chrono::{DateTime, Utc, Duration};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tracing::{info, warn, error, debug};
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
        let db_pool = SqlitePool::connect(database_url).await?;
        
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
        
        // Process CSV line by line to avoid memory issues
        let bytes = response.bytes().await?;
        let mut rdr = csv::Reader::from_reader(bytes.as_ref());
        let mut processed_count = 0;
        
        for result in rdr.deserialize() {
            let record: GeneDependencyCsvRow = result?;
            let gene_dep = GeneDependency::from(record);
            
            self.store_gene_dependency(&gene_dep).await?;
            processed_count += 1;
            
            if processed_count % 1000 == 0 {
                debug!("Processed {} gene dependency records...", processed_count);
            }
        }
        
        info!("Cached {} gene dependency records", processed_count);
        Ok(())
    }
    
    pub async fn get_releases(&self, filter: Option<&str>) -> Result<Vec<Release>> {
        let mut releases = if let Some(f) = filter {
            sqlx::query_as!(
                Release,
                "SELECT * FROM releases WHERE name LIKE ? ORDER BY release_date DESC",
                format!("%{}%", f)
            )
            .fetch_all(&self.db_pool)
            .await?
        } else {
            sqlx::query_as!(
                Release,
                "SELECT * FROM releases ORDER BY release_date DESC"
            )
            .fetch_all(&self.db_pool)
            .await?
        };
        
        // Load files for each release
        for release in &mut releases {
            release.files = self.get_release_files(&release.id).await?;
        }
        
        Ok(releases)
    }
    
    pub async fn get_datasets(&self, data_type: Option<&str>) -> Result<Vec<Dataset>> {
        let datasets = if let Some(dt) = data_type {
            sqlx::query_as!(
                Dataset,
                "SELECT * FROM datasets WHERE data_type = ? ORDER BY display_name",
                dt
            )
            .fetch_all(&self.db_pool)
            .await?
        } else {
            sqlx::query_as!(
                Dataset,
                "SELECT * FROM datasets ORDER BY data_type, display_name"
            )
            .fetch_all(&self.db_pool)
            .await?
        };
        
        Ok(datasets)
    }
    
    pub async fn search_cell_lines(&self, query: &str) -> Result<Vec<CellLine>> {
        let cell_lines = sqlx::query_as!(
            CellLine,
            "SELECT * FROM cell_lines WHERE name LIKE ? OR lineage LIKE ? OR tissue LIKE ? LIMIT 100",
            format!("%{}%", query),
            format!("%{}%", query),
            format!("%{}%", query)
        )
        .fetch_all(&self.db_pool)
        .await?;
        
        Ok(cell_lines)
    }
    
    pub async fn search_datasets(&self, query: &str) -> Result<Vec<Dataset>> {
        let datasets = sqlx::query_as!(
            Dataset,
            "SELECT * FROM datasets WHERE display_name LIKE ? OR data_type LIKE ? LIMIT 100",
            format!("%{}%", query),
            format!("%{}%", query)
        )
        .fetch_all(&self.db_pool)
        .await?;
        
        Ok(datasets)
    }
    
    pub async fn get_gene_dependencies(&self, gene: &str) -> Result<Vec<GeneDependency>> {
        let dependencies = sqlx::query_as!(
            GeneDependency,
            "SELECT * FROM gene_dependencies WHERE gene LIKE ? ORDER BY dependent_cell_lines DESC",
            format!("%{}%", gene)
        )
        .fetch_all(&self.db_pool)
        .await?;
        
        Ok(dependencies)
    }
    
    // Helper methods
    async fn store_release(&self, release: &Release) -> Result<()> {
        sqlx::query!(
            "INSERT OR REPLACE INTO releases (id, name, release_date, is_current, created_at) VALUES (?, ?, ?, ?, ?)",
            release.id,
            release.name,
            release.release_date.map(|d| d.to_rfc3339()),
            release.is_current,
            release.created_at.map(|d| d.to_rfc3339())
        )
        .execute(&self.db_pool)
        .await?;
        
        // Store files
        for file in &release.files {
            self.store_file(file).await?;
        }
        
        Ok(())
    }
    
    async fn store_dataset(&self, dataset: &Dataset) -> Result<()> {
        sqlx::query!(
            "INSERT OR REPLACE INTO datasets (id, display_name, data_type, download_entry_url, created_at) VALUES (?, ?, ?, ?, ?)",
            dataset.id,
            dataset.display_name,
            dataset.data_type,
            dataset.download_entry_url,
            dataset.created_at.map(|d| d.to_rfc3339())
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(())
    }
    
    async fn store_file(&self, file: &DownloadFile) -> Result<()> {
        sqlx::query!(
            "INSERT OR REPLACE INTO files (filename, url, md5_hash, size, release_id, data_type, is_downloaded, download_path, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            file.filename,
            file.url,
            file.md5_hash,
            file.size.map(|s| s as i64),
            file.release_id,
            file.data_type,
            file.is_downloaded,
            file.download_path,
            file.created_at.map(|d| d.to_rfc3339())
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(())
    }
    
    async fn store_gene_dependency(&self, gene_dep: &GeneDependency) -> Result<()> {
        sqlx::query!(
            "INSERT OR REPLACE INTO gene_dependencies (entrez_id, gene, dataset, dependent_cell_lines, cell_lines_with_data, strongly_selective, common_essential, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            gene_dep.entrez_id as i64,
            gene_dep.gene,
            gene_dep.dataset,
            gene_dep.dependent_cell_lines,
            gene_dep.cell_lines_with_data,
            gene_dep.strongly_selective,
            gene_dep.common_essential,
            gene_dep.created_at.map(|d| d.to_rfc3339())
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_release_files(&self, release_id: &str) -> Result<Vec<DownloadFile>> {
        let files = sqlx::query_as!(
            DownloadFile,
            "SELECT * FROM files WHERE release_id = ?",
            release_id
        )
        .fetch_all(&self.db_pool)
        .await?;
        
        Ok(files)
    }
    
    async fn is_cache_expired(&self) -> Result<bool> {
        let result = sqlx::query!(
            "SELECT updated_at FROM cache_metadata WHERE key = 'last_full_update'"
        )
        .fetch_optional(&self.db_pool)
        .await?;
        
        if let Some(row) = result {
            if let Some(updated_str) = &row.updated_at {
                let updated: DateTime<Utc> = DateTime::parse_from_rfc3339(updated_str)?.into();
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
        sqlx::query!(
            "INSERT OR REPLACE INTO cache_metadata (key, value, updated_at) VALUES ('last_full_update', 'completed', ?)",
            Utc::now().to_rfc3339()
        )
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
        
        let last_updated = sqlx::query!(
            "SELECT updated_at FROM cache_metadata WHERE key = 'last_full_update'"
        )
        .fetch_optional(&self.db_pool)
        .await?
        .and_then(|row| row.updated_at.as_deref())
        .and_then(|dt| DateTime::parse_from_rfc3339(dt).ok())
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
    
    pub async fn get_file_by_name(&self, filename: &str) -> Result<DownloadFile> {
        let file = sqlx::query_as!(
            DownloadFile,
            "SELECT * FROM files WHERE filename = ?",
            filename
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| DepMapError::FileNotFound(filename.to_string()))?;
        
        Ok(file)
    }
    
    pub async fn get_dataset_files(&self, dataset_id: &str) -> Result<Vec<DownloadFile>> {
        let files = sqlx::query_as!(
            DownloadFile,
            "SELECT f.* FROM files f 
             JOIN releases r ON f.release_id = r.id 
             WHERE r.name LIKE '%' || ? || '%' OR f.filename LIKE '%' || ? || '%'",
            dataset_id, dataset_id
        )
        .fetch_all(&self.db_pool)
        .await?;
        
        Ok(files)
    }
    
    pub async fn get_current_release_core_files(&self) -> Result<Vec<DownloadFile>> {
        let files = sqlx::query_as!(
            DownloadFile,
            "SELECT f.* FROM files f 
             JOIN releases r ON f.release_id = r.id 
             WHERE r.is_current = TRUE AND (
                 f.data_type IN ('CRISPR', 'Expression', 'Mutations', 'CN') OR
                 f.filename LIKE '%GeneEffect%' OR
                 f.filename LIKE '%Model%'
             )"
        )
        .fetch_all(&self.db_pool)
        .await?;
        
        Ok(files)
    }
}
