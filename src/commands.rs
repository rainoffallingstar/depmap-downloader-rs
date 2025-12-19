use crate::cache_manager::CacheManager;
use crate::cli::Commands;
use crate::downloader::Downloader;
use crate::error::{DepMapError, Result};
use crate::models::*;
use colored::*;
use std::path::PathBuf;
use tracing::{info, warn, error};

pub async fn handle_command(
    command: Commands,
    database_url: &str,
    api_url: &str,
    cache_dir: &PathBuf,
    _verbose: bool,
) -> Result<()> {
    let cache = CacheManager::new(database_url, api_url, cache_dir).await?;
    
    match command {
        Commands::Update { force, data_type } => {
            handle_update(&cache, force, data_type).await?;
        }
        Commands::List { release, data_type, detailed } => {
            handle_list(&cache, release, data_type, detailed).await?;
        }
        Commands::Download { dataset, file, output, workers, skip_existing, verify_checksum } => {
            handle_download(&cache, dataset, file, output, workers, skip_existing, verify_checksum).await?;
        }
        Commands::Search { query, cell_line, dataset, limit } => {
            handle_search(&cache, &query, cell_line, dataset, limit).await?;
        }
        Commands::Stats { detailed } => {
            handle_stats(&cache, detailed).await?;
        }
        Commands::Clear { all, data_type } => {
            handle_clear(&cache, all, data_type).await?;
        }
    }
    
    Ok(())
}

async fn handle_update(
    cache: &CacheManager,
    force: bool,
    data_type_filter: Option<Vec<String>>,
) -> Result<()> {
    println!("{}", "🔄 Updating DepMap cache...".bright_blue());
    
    if let Some(types) = data_type_filter {
        println!("Updating specific data types: {:?}", types);
        // TODO: Implement selective updates
        warn!("Selective data type updates not yet implemented, updating all...");
    }
    
    cache.update_cache(force).await?;
    
    println!("{}", "✅ Cache updated successfully!".bright_green());
    Ok(())
}

async fn handle_list(
    cache: &CacheManager,
    release: Option<String>,
    data_type: Option<String>,
    detailed: bool,
) -> Result<()> {
    if let Some(release_name) = release {
        println!("{}\n", format!("📁 Files for release: {}", release_name).bright_cyan());
        let releases = cache.get_releases(Some(&release_name)).await?;
        
        if releases.is_empty() {
            println!("{} No releases found matching '{}'", "⚠️".yellow(), release_name);
            return Ok(());
        }
        
        for release in releases {
            println!("{}", format!("Release: {} ({})", release.name, 
                release.release_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "Unknown".to_string())
            ).bold());
            
            if detailed {
                println!("  Files:");
            }
            
            for file in release.files {
                if detailed {
                    println!("    📄 {} ({})", file.filename.bright_green(), 
                        file.data_type.as_deref().unwrap_or("Unknown").italic());
                    if let Some(size) = file.size {
                        println!("      Size: {} MB", size / (1024 * 1024));
                    }
                    if file.is_downloaded {
                        println!("      ✅ Downloaded");
                    }
                } else {
                    println!("  📄 {}", file.filename.bright_green());
                }
            }
            println!();
        }
    } else if let Some(d_type) = data_type {
        println!("{}\n", format!("🔬 Datasets of type: {}", d_type).bright_cyan());
        let datasets = cache.get_datasets(Some(&d_type)).await?;
        
        if datasets.is_empty() {
            println!("{} No datasets found of type '{}'", "⚠️".yellow(), d_type);
            return Ok(());
        }
        
        for dataset in datasets {
            if detailed {
                println!("📊 {} ({})", dataset.display_name.bold(), dataset.data_type.italic());
                if let Some(url) = dataset.download_entry_url {
                    println!("   🔗 {}", url);
                }
            } else {
                println!("📊 {}", dataset.display_name.bright_green());
            }
        }
    } else {
        // List all releases
        println!("{}\n", "📦 Available Releases".bright_cyan());
        let releases = cache.get_releases(None).await?;
        
        if releases.is_empty() {
            println!("{} No releases found. Run 'update' first.", "⚠️".yellow());
            return Ok(());
        }
        
        let release_count = releases.len();
        for release in releases {
            let current_marker = if release.is_current { " 🌟" } else { "" };
            println!("{}{} ({})", 
                release.name.bold().bright_green(),
                current_marker.bright_yellow(),
                release.release_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "Unknown".to_string())
            );
            
            if detailed {
                println!("  📁 {} files", release.files.len());
                println!("  🆔 {}", release.id);
            }
            println!();
        }
        
        println!("Total: {} releases", release_count);
    }
    
    Ok(())
}

async fn handle_download(
    cache: &CacheManager,
    dataset: Option<String>,
    file: Option<String>,
    output: String,
    workers: usize,
    skip_existing: bool,
    verify_checksum: bool,
) -> Result<()> {
    let download_items = match (dataset, file) {
        (Some(dataset_id), None) => {
            println!("📥 Downloading dataset: {}", dataset_id.bright_green());
            cache.get_dataset_files(&dataset_id).await?
        }
        (None, Some(filename)) => {
            println!("📥 Downloading file: {}", filename.bright_green());
            vec![cache.get_file_by_name(&filename).await?]
        }
        (None, None) => {
            println!("📥 Downloading current release core files...");
            cache.get_current_release_core_files().await?
        }
        (Some(_), Some(_)) => {
            return Err(DepMapError::InvalidArguments);
        }
    };
    
    if download_items.is_empty() {
        println!("{} No files found to download", "⚠️".yellow());
        return Ok(());
    }
    
    println!("Found {} files to download", download_items.len());
    
    let downloader = Downloader::new(workers, output, skip_existing, verify_checksum)?;
    downloader.download_files(download_items).await?;
    
    println!("{}", "✅ Download completed!".bright_green());
    Ok(())
}

async fn handle_search(
    cache: &CacheManager,
    query: &str,
    cell_line: bool,
    dataset: bool,
    limit: Option<usize>,
) -> Result<()> {
    println!("{}\n", format!("🔍 Searching for: {}", query.bright_cyan()).bold());
    
    let limit = limit.unwrap_or(50);
    
    if cell_line || (!cell_line && !dataset) {
        println!("🧬 Searching in cell lines...");
        match cache.search_cell_lines(query).await {
            Ok(cell_lines) => {
                if cell_lines.is_empty() {
                    println!("  No cell lines found");
                } else {
                    let limited: Vec<_> = cell_lines.into_iter().take(limit).collect();
                    println!("  Found {} matching cell lines:", limited.len());
                    for cell_line in limited {
                        println!("    🧬 {} ({})", 
                            cell_line.name.bright_green(),
                            cell_line.lineage.as_deref().unwrap_or("Unknown").italic()
                        );
                        if let Some(tissue) = cell_line.tissue {
                            println!("      Tissue: {}", tissue);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to search cell lines: {}", e);
            }
        }
        println!();
    }
    
    if dataset || (!cell_line && !dataset) {
        println!("📊 Searching in datasets...");
        match cache.search_datasets(query).await {
            Ok(datasets) => {
                if datasets.is_empty() {
                    println!("  No datasets found");
                } else {
                    let limited: Vec<_> = datasets.into_iter().take(limit).collect();
                    println!("  Found {} matching datasets:", limited.len());
                    for dataset in limited {
                        println!("    📊 {} ({})", 
                            dataset.display_name.bright_green(),
                            dataset.data_type.italic()
                        );
                    }
                }
            }
            Err(e) => {
                warn!("Failed to search datasets: {}", e);
            }
        }
    }
    
    Ok(())
}

async fn handle_stats(
    cache: &CacheManager,
    detailed: bool,
) -> Result<()> {
    println!("{}\n", "📊 Cache Statistics".bright_cyan().bold());
    
    let stats = cache.get_cache_stats().await?;
    
    println!("📦 Releases: {}", stats.release_count.to_string().bright_green());
    println!("📁 Files: {}", stats.file_count.to_string().bright_green());
    println!("📊 Datasets: {}", stats.dataset_count.to_string().bright_green());
    println!("🧬 Cell Lines: {}", stats.cell_line_count.to_string().bright_green());
    println!("🧪 Gene Dependencies: {}", stats.gene_dependency_count.to_string().bright_green());
    
    if stats.total_size_mb > 0 {
        println!("💾 Total Size: {} MB", stats.total_size_mb.to_string().bright_yellow());
    }
    
    if let Some(last_updated) = stats.last_updated {
        println!("🕒 Last Updated: {}", last_updated.format("%Y-%m-%d %H:%M:%S UTC").to_string().bright_blue());
    } else {
        println!("🕒 Last Updated: {}", "Never".bright_red());
    }
    
    if detailed {
        println!("\n🔍 Detailed Information:");
        
        // Show release breakdown
        let releases = cache.get_releases(None).await?;
        println!("\n📦 Releases:");
        for release in releases {
            let marker = if release.is_current { " 🌟" } else { "" };
            println!("  {}{} ({} files)", 
                release.name.bold(),
                marker.bright_yellow(),
                release.files.len()
            );
        }
        
        // Show dataset type breakdown
        let datasets = cache.get_datasets(None).await?;
        let mut type_counts = std::collections::HashMap::new();
        for dataset in datasets {
            *type_counts.entry(dataset.data_type).or_insert(0) += 1;
        }
        
        println!("\n📊 Dataset Types:");
        for (data_type, count) in type_counts {
            println!("  {}: {}", data_type.italic(), count.to_string().bright_green());
        }
    }
    
    Ok(())
}

async fn handle_clear(
    _cache: &CacheManager,
    all: bool,
    data_type: Option<String>,
) -> Result<()> {
    if all {
        println!("{} Clearing all cached data...", "🗑️".bright_red());
        // TODO: Implement cache clearing
        println!("{}", "⚠️ Cache clearing not yet implemented".yellow());
    } else if let Some(d_type) = data_type {
        println!("{} Clearing cached data of type: {}", "🗑️".bright_red(), d_type);
        // TODO: Implement selective cache clearing
        println!("{}", "⚠️ Selective cache clearing not yet implemented".yellow());
    } else {
        println!("{}", "❌ No clear option specified. Use --all or --data-type".red());
    }
    
    Ok(())
}
