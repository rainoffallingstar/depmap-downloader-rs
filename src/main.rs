mod cache_manager;
mod cli;
mod commands;
mod downloader;
mod error;
mod models;

use clap::Parser;
use cli::Cli;
use error::Result;
use tracing::error;
use tracing_subscriber::FmtSubscriber;

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
        .with_writer(std::io::stderr)
        .init();

    // Create cache directory if it doesn't exist
    let cache_dir = std::path::PathBuf::from(".depmap_cache");

    // Handle the command
    if let Err(e) = commands::handle_command(
        cli.command,
        &cli.database,
        &cli.api_url,
        &cache_dir,
        cli.json,
        cli.verbose,
    )
    .await
    {
        error!("Command failed: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
