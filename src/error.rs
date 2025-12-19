use thiserror::Error;

#[derive(Debug, Error)]
pub enum DepMapError {
    #[error("API request failed: {0}")]
    ApiError(#[from] reqwest::Error),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Invalid arguments")]
    InvalidArguments,
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("CSV parsing error: {0}")]
    CsvError(#[from] csv::Error),
    
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Checksum validation failed for {0}")]
    ChecksumError(String),
    
    #[error("Download failed: {0}")]
    DownloadError(String),
    
    #[error("Chrono parse error: {0}")]
    ChronoError(#[from] chrono::ParseError),
    
    #[error("Semaphore acquire error: {0}")]
    SemaphoreError(#[from] tokio::sync::AcquireError),
    
    #[error("Task join error: {0}")]
    TaskJoinError(#[from] tokio::task::JoinError),
}

pub type Result<T> = std::result::Result<T, DepMapError>;
