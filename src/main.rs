mod models;
mod error;
mod cli;
mod cache_manager;
mod downloader;
mod commands;

use clap::Parser;
use error::{Result, DepMapError};
use tracing::{info, warn, error};
use tracing_subscriber::FmtSubscriber;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .init();
    
    // Create cache directory if it doesn't exist
    let cache_dir = std::path::PathBuf::from(".depmap_cache");
    
    // Handle the command
    if let Err(e) = commands::handle_command(
        cli.command,
        &cli.database,
        &cli.api_url,
        &cache_dir,
        cli.verbose,
    ).await {
        error!("Command failed: {}", e);
        std::process::exit(1);
    }
    
    Ok(())
}
