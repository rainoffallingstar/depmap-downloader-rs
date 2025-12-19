use crate::cache_manager::CacheManager;
use crate::cli::{Commands, ListCommands, DownloadCommands};
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
        Commands::List { command } => {
            handle_list_command(&cache, command).await?;
        }
        Commands::Download { command, dataset, file, output, workers, skip_existing, verify_checksum } => {
            handle_download(&cache, command, dataset, file, output, workers, skip_existing, verify_checksum).await?;
        }
        Commands::Search { query, cell_line, gene, dataset, limit } => {
            handle_search(&cache, &query, cell_line, gene, dataset, limit).await?;
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
        cache.update_cache_selective(force, types).await?;
    } else {
        cache.update_cache(force).await?;
    }
    
    println!("{}", "✅ Cache updated successfully!".bright_green());
    Ok(())
}

async fn handle_list_command(
    cache: &CacheManager,
    command: Option<ListCommands>,
) -> Result<()> {
    match command {
        Some(ListCommands::Releases { detailed }) => {
            handle_list_releases(cache, detailed).await?;
        }
        Some(ListCommands::Datasets { data_type, detailed }) => {
            handle_list_datasets(cache, data_type, detailed).await?;
        }
        Some(ListCommands::Files { release, detailed }) => {
            handle_list_files(cache, &release, detailed).await?;
        }
        None => {
            // Default behavior: show overview
            handle_list_overview(cache).await?;
        }
    }
    
    Ok(())
}

async fn handle_list_overview(cache: &CacheManager) -> Result<()> {
    println!("{}", "📦 DepMap Data Overview".bright_cyan().bold());
    println!("{}", "─".repeat(50).dimmed());
    
    // Get releases
    let releases = cache.get_releases(None).await?;
    println!("\n{}", format!("📦 Available Releases ({})", releases.len()).bright_green().bold());
    
    // Show recent releases (up to 10)
    let mut recent_releases = releases.clone();
    recent_releases.sort_by(|a, b| b.release_date.cmp(&a.release_date));
    recent_releases.truncate(10);
    
    for (i, release) in recent_releases.iter().enumerate() {
        let is_current = release.is_current || release.name.contains("25Q3");
        let indicator = if is_current { "🌟" } else { "  " };
        let date_str = release.release_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        println!("  {} {} ({})", indicator, release.name.bright_white(), date_str.dimmed());
    }
    
    if releases.len() > 10 {
        println!("  {} ... and {} more releases", "   ".dimmed(), releases.len() - 10);
    }
    
    // Get datasets
    let datasets = cache.get_datasets(None).await?;
    
    // Group datasets by type
    let mut dataset_types = std::collections::HashMap::new();
    for dataset in &datasets {
        dataset_types.entry(dataset.data_type.clone())
            .or_insert_with(Vec::new)
            .push(dataset);
    }
    
    println!("\n{}", format!("📊 Available Dataset Types ({})", dataset_types.len()).bright_green().bold());
    
    let mut types: Vec<_> = dataset_types.keys().collect();
    types.sort();
    
    for data_type in types {
        let datasets_of_type = &dataset_types[data_type];
        if datasets_of_type.len() == 1 {
            println!("  🔬 {} (1 dataset)", data_type.bright_white());
        } else {
            println!("  🔬 {} ({} datasets)", data_type.bright_white(), datasets_of_type.len());
        }
        
        // Show a few example dataset names
        for (i, dataset) in datasets_of_type.iter().take(2).enumerate() {
            println!("    • {}", dataset.display_name.dimmed());
        }
        
        if datasets_of_type.len() > 2 {
            println!("    • ... and {} more", datasets_of_type.len() - 2);
        }
    }
    
    println!("\n{}", "💡 Use 'list releases', 'list datasets', or 'list files <release>' for more details".bright_yellow());
    
    Ok(())
}

async fn handle_list_releases(cache: &CacheManager, detailed: bool) -> Result<()> {
    println!("{}", "📦 DepMap Releases".bright_cyan().bold());
    println!("{}", "─".repeat(50).dimmed());
    
    let releases = cache.get_releases(None).await?;
    
    if releases.is_empty() {
        println!("{} No releases found", "⚠️".yellow());
        return Ok(());
    }
    
    // Sort releases by date (newest first)
    let mut sorted_releases = releases;
    sorted_releases.sort_by(|a, b| b.release_date.cmp(&a.release_date));
    
    for release in sorted_releases {
        let is_current = release.is_current || release.name.contains("25Q3");
        let current_indicator = if is_current { " 🌟 CURRENT" } else { "" };
        let date_str = release.release_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        println!("{}", format!("📦 {}{} ({})", release.name.bold(), current_indicator.bright_green(), date_str.dimmed()));
        
        if detailed {
            println!("  🆔 ID: {}", release.id.italic());
            println!("  📁 Files: {}", release.files.len().to_string().bright_blue());
            println!("  🕒 Created: {}", release.created_at
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
                .dimmed());
        }
        
        println!();
    }
    
    Ok(())
}

async fn handle_list_datasets(cache: &CacheManager, data_type_filter: Option<String>, detailed: bool) -> Result<()> {
    println!("{}", "📊 DepMap Datasets".bright_cyan().bold());
    println!("{}", "─".repeat(50).dimmed());
    
    let datasets = cache.get_datasets(data_type_filter.as_deref()).await?;
    
    if datasets.is_empty() {
        let filter_msg = data_type_filter
            .map(|t| format!(" of type '{}'", t))
            .unwrap_or_else(|| "".to_string());
        println!("{} No datasets found{}", "⚠️".yellow(), filter_msg);
        return Ok(());
    }
    
    // Group datasets by type
    let mut dataset_groups = std::collections::HashMap::new();
    for dataset in &datasets {
        dataset_groups.entry(dataset.data_type.clone())
            .or_insert_with(Vec::new)
            .push(dataset);
    }
    
    let mut types: Vec<_> = dataset_groups.keys().collect();
    types.sort();
    
    for data_type in types {
        let datasets_of_type = &dataset_groups[data_type];
        
        println!("{}", format!("🔬 {} ({} datasets)", data_type.bright_green().bold(), datasets_of_type.len()));
        
        for dataset in datasets_of_type {
            println!("  📋 {}", dataset.display_name.bright_white());
            
            if detailed {
                println!("    🆔 ID: {}", dataset.id.italic());
                if let Some(url) = &dataset.download_entry_url {
                    println!("    🔗 URL: {}", url.dimmed());
                }
                println!("    🕒 Created: {}", dataset.created_at
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
                    .dimmed());
            }
        }
        println!();
    }
    
    Ok(())
}

async fn handle_list_files(cache: &CacheManager, release_name: &str, detailed: bool) -> Result<()> {
    println!("{}", format!("📁 Files for release: {}", release_name).bright_cyan().bold());
    println!("{}", "─".repeat(50).dimmed());
    
    let releases = cache.get_releases(Some(release_name)).await?;
    
    if releases.is_empty() {
        println!("{} No releases found matching '{}'", "⚠️".yellow(), release_name);
        return Ok(());
    }
    
    for release in releases {
        println!("{}", format!("📦 Release: {} ({})", release.name.bright_white(), 
            release.release_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "Unknown".to_string())
        ));
        
        if detailed {
            println!("  📁 Files ({} total):", release.files.len());
        }
        
        // Group files by data type
        let mut files_by_type = std::collections::HashMap::new();
        for file in &release.files {
            let data_type = file.data_type.as_deref().unwrap_or("Unknown");
            files_by_type.entry(data_type.to_string())
                .or_insert_with(Vec::new)
                .push(file);
        }
        
        let mut types: Vec<_> = files_by_type.keys().collect();
        types.sort();
        
        for data_type in types {
            let files_of_type = &files_by_type[data_type];
            
            println!("\n  {} {} ({} files)", if detailed { "🔬" } else { "  🔬" }, data_type.bright_green(), files_of_type.len());
            
            for file in files_of_type {
                if detailed {
                    let status = if file.is_downloaded { "✅" } else { "⬜" };
                    let size_mb = file.size.map(|s| format!("{} MB", s / (1024 * 1024))).unwrap_or_else(|| "Unknown size".to_string());
                    
                    println!("    {} {} ({})", status, file.filename.bright_white(), size_mb.dimmed());
                    println!("      🆔 ID: {}", file.id.unwrap_or(0));
                    println!("      🔗 URL: {}", file.url.chars().take(80).collect::<String>() + "...");
                    
                    if let Some(hash) = &file.md5_hash {
                        println!("      🔐 MD5: {}", &hash[..8]);
                    }
                } else {
                    println!("    📄 {}", file.filename.bright_white());
                }
            }
        }
        
        println!();
    }
    
    Ok(())
}

// Legacy function for backward compatibility
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
    command: Option<DownloadCommands>,
    dataset: Option<String>,
    file: Option<String>,
    output: String,
    workers: usize,
    skip_existing: bool,
    verify_checksum: bool,
) -> Result<()> {
    let download_items = match command {
        Some(DownloadCommands::Release { name, data_type }) => {
            println!("📥 Downloading release: {}", name.bright_green());
            cache.get_release_files(&name, data_type.as_deref()).await?
        }
        Some(DownloadCommands::Dataset { id }) => {
            println!("📥 Downloading dataset: {}", id.bright_green());
            cache.get_dataset_files(&id).await?
        }
        None => {
            // Backward compatibility with old arguments
            match (dataset, file) {
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
            }
        }
    };
    
    if download_items.is_empty() {
        println!("{} No files found to download", "⚠️".yellow());
        println!("💡 Try running 'update' first to refresh the cache");
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
    gene: bool,
    dataset: bool,
    limit: Option<usize>,
) -> Result<()> {
    println!("{}\n", format!("🔍 Searching for: {}", query.bright_cyan()).bold());
    
    let limit = limit.unwrap_or(50);
    
    if gene || (!cell_line && !gene && !dataset) {
        println!("🧬 Searching in genes...");
        match cache.search_genes(query, limit).await {
            Ok(genes) => {
                if genes.is_empty() {
                    println!("  No genes found");
                } else {
                    println!("  Found {} matching genes:", genes.len());
                    for gene in genes {
                        let essential = if gene.common_essential {
                            "Common Essential".bright_red()
                        } else if gene.strongly_selective {
                            "Strongly Selective".bright_yellow()
                        } else {
                            "Non-essential".bright_green()
                        };
                        
                        println!("    🧬 {} (Entrez ID: {})", gene.gene.bright_white(), gene.entrez_id.to_string().dimmed());
                        println!("      📊 Dataset: {}", gene.dataset.italic());
                        println!("      📈 Dependent Cell Lines: {}", gene.dependent_cell_lines);
                        println!("      🧪 Cell Lines with Data: {}", gene.cell_lines_with_data);
                        println!("      ⭐ Status: {}", essential);
                        println!();
                    }
                }
            }
            Err(e) => {
                warn!("Failed to search genes: {}", e);
            }
        }
    }
    
    if cell_line || (!cell_line && !gene && !dataset) {
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
    
    if dataset || (!cell_line && !gene && !dataset) {
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
    cache: &CacheManager,
    all: bool,
    data_type: Option<String>,
) -> Result<()> {
    if all {
        println!("{} Clearing all cached data...", "🗑️".bright_red());
        cache.clear_all_cache().await?;
        println!("{}", "✅ All cached data cleared successfully!".bright_green());
    } else if let Some(d_type) = data_type {
        println!("{} Clearing cached data of type: {}", "🗑️".bright_red(), d_type);
        let deleted_count = cache.clear_cache_by_data_type(&d_type).await?;
        println!("{}", format!("✅ Cleared {} files of type '{}'!", deleted_count, d_type).bright_green());
    } else {
        println!("{}", "❌ No clear option specified. Use --all or --data-type".red());
    }
    
    Ok(())
}
