use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "depdown")]
#[command(about = "A Rust-based DepMap data downloader")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(
        long,
        global = true,
        help = "Emit machine-readable JSON to stdout (best-effort for all commands)"
    )]
    pub json: bool,

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

        #[arg(
            short,
            long,
            default_value_t = 4,
            help = "Number of concurrent downloads"
        )]
        workers: usize,

        #[arg(long, help = "Skip existing files")]
        skip_existing: bool,

        #[arg(long, help = "Verify checksum after download")]
        verify_checksum: bool,

        #[arg(
            long = "no-refresh-links",
            action = ArgAction::SetFalse,
            default_value_t = true,
            help = "Disable refreshing file URLs from DepMap API before download"
        )]
        refresh_links: bool,

        #[arg(
            long,
            help = "Fail preparation if URL refresh cannot resolve fresh links for files"
        )]
        strict_refresh: bool,
    },
    /// Alignment workflows for cellline+intervention artifacts
    Align {
        #[command(subcommand)]
        command: AlignCommands,
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
    /// Download files listed in a manifest CSV
    Manifest {
        #[arg(long, help = "Path to manifest CSV")]
        file: String,

        #[arg(long, help = "Only download required_for_alignment=Y rows")]
        required_only: bool,

        #[arg(long, default_value_t = 0, help = "Retry attempts for failed files")]
        retry: usize,
    },
    /// Reconcile local files from an external downloaded directory into output dir
    ReconcileLocal {
        #[arg(
            long,
            default_value = "depmap-compliment/depmap_data",
            help = "Source directory with manually downloaded files"
        )]
        from: String,

        #[arg(
            long,
            default_value = "depmap_data",
            help = "Target directory used by downloader/alignment"
        )]
        to: String,

        #[arg(
            long,
            default_value = "alignment/download_failures.csv",
            help = "Manifest CSV containing filename column"
        )]
        manifest: String,

        #[arg(long, help = "Allow replacing non-zero files in target directory")]
        overwrite_nonzero: bool,

        #[arg(
            long,
            default_value_t = true,
            action = ArgAction::Set,
            help = "Require source files to be non-zero"
        )]
        verify_size_nonzero: bool,
    },
}

#[derive(Subcommand)]
pub enum AlignCommands {
    /// Build the cellline+intervention index artifacts
    BuildIndex,
    /// Build long-format multimodal fact tables
    BuildLongTable {
        #[arg(
            long,
            help = "Comma-separated assays (default: all supported assays found on disk)"
        )]
        assays: Option<String>,
    },
}
