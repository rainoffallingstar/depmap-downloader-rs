use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "depmap-downloader")]
#[command(about = "A Rust-based DepMap data downloader")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    #[arg(short, long, default_value = "depmap_cache.db")]
    pub database: String,
    
    #[arg(short, long, default_value = "https://depmap.org/portal/api")]
    pub api_url: String,
    
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Update local cache from DepMap API
    Update {
        #[arg(short, long, help = "Force update even if cache is recent")]
        force: bool,
        
        #[arg(short, long, help = "Update specific data types only")]
        data_type: Option<Vec<String>>,
    },
    /// List available releases, datasets, or files
    List {
        #[command(subcommand)]
        command: Option<ListCommands>,
    },
    /// Download specific datasets or files
    Download {
        #[command(subcommand)]
        command: Option<DownloadCommands>,
        
        // 向后兼容支持
        #[arg(short, long, help = "Download specific dataset by ID")]
        dataset: Option<String>,
        
        #[arg(short, long, help = "Download specific file by name")]
        file: Option<String>,
        
        #[arg(short, long, default_value = "depmap_data", help = "Output directory")]
        output: String,
        
        #[arg(short, long, default_value_t = 4, help = "Number of concurrent downloads")]
        workers: usize,
        
        #[arg(long, help = "Skip existing files")]
        skip_existing: bool,
        
        #[arg(long, help = "Verify checksum after download")]
        verify_checksum: bool,
    },
    /// Search for cell lines, genes, or datasets
    Search {
        query: String,
        
        #[arg(short, long, help = "Search in cell lines")]
        cell_line: bool,
        
        #[arg(short, long, help = "Search in genes")]
        gene: bool,
        
        #[arg(short, long, help = "Search in datasets")]
        dataset: bool,
        
        #[arg(short = 't', long, help = "Limit number of results")]
        limit: Option<usize>,
    },
    /// Show cache statistics
    Stats {
        #[arg(long, help = "Show detailed statistics")]
        detailed: bool,
    },
    /// Clear cache
    Clear {
        #[arg(long, help = "Clear all cached data")]
        all: bool,
        
        #[arg(short, long, help = "Clear specific data type")]
        data_type: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ListCommands {
    /// List all available releases
    Releases {
        #[arg(long, help = "Show detailed information")]
        detailed: bool,
    },
    /// List available datasets
    Datasets {
        #[arg(long, short = 't', help = "Filter by dataset type")]
        data_type: Option<String>,
        
        #[arg(long, help = "Show detailed information")]
        detailed: bool,
    },
    /// List files in a specific release
    Files {
        #[arg(help = "Release name")]
        release: String,
        
        #[arg(long, help = "Show detailed information")]
        detailed: bool,
    },
}

#[derive(Subcommand)]
pub enum DownloadCommands {
    /// Download all files from a specific release
    Release {
        #[arg(help = "Release name (e.g., 23Q4, 24Q1)")]
        name: String,
        
        #[arg(long, help = "Filter by data type")]
        data_type: Option<String>,
    },
    /// Download all files for a specific dataset
    Dataset {
        #[arg(help = "Dataset ID")]
        id: String,
    },
}
