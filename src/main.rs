fn main() {
  mod models;
mod error;
mod cli;
mod release_manager;
mod downloader;
mod database;
mod utils;

use clap::Parser;
use error::{Result, DepMapError};
use tracing::{info, warn, error};
use tracing_subscriber::FmtSubscriber;
use std::path::PathBuf;

use cli::{Cli, Commands};
}
