use crate::cache_manager::CacheManager;
use crate::cli::{AlignCommands, Commands, DownloadCommands, ListCommands};
use crate::downloader::Downloader;
use crate::error::{DepMapError, Result};
use crate::models::*;
use colored::*;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tracing::warn;

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    use std::io::{ErrorKind, Write};

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    match serde_json::to_writer(&mut handle, value) {
        Ok(()) => {}
        Err(e) => {
            if e.io_error_kind() == Some(ErrorKind::BrokenPipe) {
                return Ok(());
            }
            return Err(e.into());
        }
    }

    if let Err(e) = writeln!(&mut handle) {
        if e.kind() == ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(e.into());
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct ReleaseSummary {
    id: String,
    name: String,
    release_date: Option<String>,
    is_current: bool,
    files_count: usize,
}

#[derive(Debug, Serialize)]
struct DatasetSummary {
    id: String,
    display_name: String,
    data_type: String,
    download_entry_url: Option<String>,
    created_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct FileSummary {
    filename: String,
    url: String,
    md5_hash: Option<String>,
    size: Option<u64>,
    data_type: Option<String>,
    release_id: Option<String>,
    is_downloaded: bool,
    download_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct DatasetTypeSummary {
    data_type: String,
    dataset_count: usize,
    examples: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ListOverviewJson {
    releases_total: usize,
    releases_recent: Vec<ReleaseSummary>,
    datasets_total: usize,
    dataset_types: Vec<DatasetTypeSummary>,
}

pub async fn handle_command(
    command: Commands,
    database_url: &str,
    api_url: &str,
    cache_dir: &PathBuf,
    json: bool,
    _verbose: bool,
) -> Result<()> {
    let cache = CacheManager::new(database_url, api_url, cache_dir).await?;

    match command {
        Commands::Update { force, data_type } => {
            handle_update(&cache, force, data_type).await?;
        }
        Commands::List { command } => {
            handle_list_command(&cache, command, json).await?;
        }
        Commands::Download {
            command,
            dataset,
            file,
            output,
            workers,
            skip_existing,
            verify_checksum,
            refresh_links,
            strict_refresh,
        } => {
            handle_download(
                &cache,
                command,
                dataset,
                file,
                output,
                workers,
                skip_existing,
                verify_checksum,
                refresh_links,
                strict_refresh,
            )
            .await?;
        }
        Commands::Align { command } => {
            handle_align(command).await?;
        }
        Commands::Search {
            query,
            cell_line,
            gene,
            dataset,
            limit,
        } => {
            handle_search(&cache, &query, cell_line, gene, dataset, limit, json).await?;
        }
        Commands::Stats { detailed } => {
            handle_stats(&cache, detailed, json).await?;
        }
        Commands::Clear { all, data_type } => {
            handle_clear(&cache, all, data_type).await?;
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct ManifestRow {
    filename: String,
    #[serde(default)]
    release_id: Option<String>,
    #[serde(default)]
    dataset_id: Option<String>,
    #[serde(default)]
    required_for_alignment: Option<String>,
}

fn flag_is_yes(value: Option<&str>) -> bool {
    value
        .map(|v| {
            matches!(
                v.trim().to_ascii_uppercase().as_str(),
                "Y" | "YES" | "TRUE" | "1"
            )
        })
        .unwrap_or(false)
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
    json: bool,
) -> Result<()> {
    match command {
        Some(ListCommands::Releases { detailed }) => {
            handle_list_releases(cache, detailed, json).await?;
        }
        Some(ListCommands::Datasets {
            data_type,
            detailed,
        }) => {
            handle_list_datasets(cache, data_type, detailed, json).await?;
        }
        Some(ListCommands::Files { release, detailed }) => {
            handle_list_files(cache, &release, detailed, json).await?;
        }
        None => {
            // Default behavior: show overview
            handle_list_overview(cache, json).await?;
        }
    }

    Ok(())
}

async fn handle_list_overview(cache: &CacheManager, json: bool) -> Result<()> {
    // Get releases
    let releases = cache.get_releases(None).await?;
    let current_release_id = releases
        .iter()
        .max_by_key(|r| r.release_date)
        .map(|r| r.id.clone());

    if json {
        let mut recent_releases = releases.clone();
        recent_releases.sort_by(|a, b| b.release_date.cmp(&a.release_date));
        recent_releases.truncate(10);

        let releases_recent: Vec<ReleaseSummary> = recent_releases
            .into_iter()
            .map(|r| ReleaseSummary {
                id: r.id.clone(),
                name: r.name.clone(),
                release_date: r.release_date.as_ref().map(|d| d.to_rfc3339()),
                is_current: current_release_id.as_deref().is_some_and(|id| id == r.id),
                files_count: r.files.len(),
            })
            .collect();

        let datasets = cache.get_datasets(None).await?;
        let mut dataset_types_map: HashMap<String, Vec<&Dataset>> = HashMap::new();
        for dataset in &datasets {
            dataset_types_map
                .entry(dataset.data_type.clone())
                .or_default()
                .push(dataset);
        }

        let mut dataset_types: Vec<DatasetTypeSummary> = dataset_types_map
            .into_iter()
            .map(|(data_type, group)| DatasetTypeSummary {
                dataset_count: group.len(),
                examples: group
                    .iter()
                    .take(2)
                    .map(|d| d.display_name.clone())
                    .collect(),
                data_type,
            })
            .collect();
        dataset_types.sort_by(|a, b| a.data_type.cmp(&b.data_type));

        return print_json(&ListOverviewJson {
            releases_total: releases.len(),
            releases_recent,
            datasets_total: datasets.len(),
            dataset_types,
        });
    }

    println!("{}", "📦 DepMap Data Overview".bright_cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    println!(
        "\n{}",
        format!("📦 Available Releases ({})", releases.len())
            .bright_green()
            .bold()
    );

    // Show recent releases (up to 10)
    let mut recent_releases = releases.clone();
    recent_releases.sort_by(|a, b| b.release_date.cmp(&a.release_date));
    recent_releases.truncate(10);

    for release in recent_releases.iter() {
        let is_current = current_release_id
            .as_deref()
            .is_some_and(|id| id == release.id);
        let indicator = if is_current { "🌟" } else { "  " };
        let date_str = release
            .release_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        println!(
            "  {} {} ({})",
            indicator,
            release.name.bright_white(),
            date_str.dimmed()
        );
    }

    if releases.len() > 10 {
        println!(
            "  {} ... and {} more releases",
            "   ".dimmed(),
            releases.len() - 10
        );
    }

    // Get datasets
    let datasets = cache.get_datasets(None).await?;

    // Group datasets by type
    let mut dataset_types = std::collections::HashMap::new();
    for dataset in &datasets {
        dataset_types
            .entry(dataset.data_type.clone())
            .or_insert_with(Vec::new)
            .push(dataset);
    }

    println!(
        "\n{}",
        format!("📊 Available Dataset Types ({})", dataset_types.len())
            .bright_green()
            .bold()
    );

    let mut types: Vec<_> = dataset_types.keys().collect();
    types.sort();

    for data_type in types {
        let datasets_of_type = &dataset_types[data_type];
        if datasets_of_type.len() == 1 {
            println!("  🔬 {} (1 dataset)", data_type.bright_white());
        } else {
            println!(
                "  🔬 {} ({} datasets)",
                data_type.bright_white(),
                datasets_of_type.len()
            );
        }

        // Show a few example dataset names
        for dataset in datasets_of_type.iter().take(2) {
            println!("    • {}", dataset.display_name.dimmed());
        }

        if datasets_of_type.len() > 2 {
            println!("    • ... and {} more", datasets_of_type.len() - 2);
        }
    }

    println!(
        "\n{}",
        "💡 Use 'list releases', 'list datasets', or 'list files <release>' for more details"
            .bright_yellow()
    );

    Ok(())
}

async fn handle_list_releases(cache: &CacheManager, detailed: bool, json: bool) -> Result<()> {
    let releases = cache.get_releases(None).await?;

    if releases.is_empty() {
        if json {
            return print_json(&serde_json::json!({ "releases": [], "detailed": detailed }));
        }
        println!("{}", "📦 DepMap Releases".bright_cyan().bold());
        println!("{}", "─".repeat(50).dimmed());
        println!("{} No releases found", "⚠️".yellow());
        return Ok(());
    }

    // Sort releases by date (newest first)
    let mut sorted_releases = releases;
    sorted_releases.sort_by(|a, b| b.release_date.cmp(&a.release_date));
    let current_release_id = sorted_releases.first().map(|r| r.id.clone());

    if json {
        let releases: Vec<ReleaseSummary> = sorted_releases
            .iter()
            .map(|r| ReleaseSummary {
                id: r.id.clone(),
                name: r.name.clone(),
                release_date: r.release_date.as_ref().map(|d| d.to_rfc3339()),
                is_current: current_release_id.as_deref().is_some_and(|id| id == r.id),
                files_count: r.files.len(),
            })
            .collect();
        return print_json(&serde_json::json!({ "releases": releases, "detailed": detailed }));
    }

    println!("{}", "📦 DepMap Releases".bright_cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    for release in sorted_releases {
        let is_current = current_release_id
            .as_deref()
            .is_some_and(|id| id == release.id);
        let current_indicator = if is_current { " 🌟 CURRENT" } else { "" };
        let date_str = release
            .release_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        println!(
            "📦 {}{} ({})",
            release.name.bold(),
            current_indicator.bright_green(),
            date_str.dimmed()
        );

        if detailed {
            println!("  🆔 ID: {}", release.id.italic());
            println!(
                "  📁 Files: {}",
                release.files.len().to_string().bright_blue()
            );
            println!(
                "  🕒 Created: {}",
                release
                    .created_at
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
                    .dimmed()
            );
        }

        println!();
    }

    Ok(())
}

async fn handle_list_datasets(
    cache: &CacheManager,
    data_type_filter: Option<String>,
    detailed: bool,
    json: bool,
) -> Result<()> {
    let datasets = cache.get_datasets(data_type_filter.as_deref()).await?;

    if json {
        let datasets: Vec<DatasetSummary> = datasets
            .iter()
            .map(|d| DatasetSummary {
                id: d.id.clone(),
                display_name: d.display_name.clone(),
                data_type: d.data_type.clone(),
                download_entry_url: d.download_entry_url.clone(),
                created_at: d.created_at.as_ref().map(|dt| dt.to_rfc3339()),
            })
            .collect();
        return print_json(&serde_json::json!({ "datasets": datasets, "detailed": detailed }));
    }

    println!("{}", "📊 DepMap Datasets".bright_cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    if datasets.is_empty() {
        let filter_msg = data_type_filter
            .map(|t| format!(" of type '{}'", t))
            .unwrap_or_default();
        println!("{} No datasets found{}", "⚠️".yellow(), filter_msg);
        return Ok(());
    }

    // Group datasets by type
    let mut dataset_groups = std::collections::HashMap::new();
    for dataset in &datasets {
        dataset_groups
            .entry(dataset.data_type.clone())
            .or_insert_with(Vec::new)
            .push(dataset);
    }

    let mut types: Vec<_> = dataset_groups.keys().collect();
    types.sort();

    for data_type in types {
        let datasets_of_type = &dataset_groups[data_type];

        println!(
            "🔬 {} ({} datasets)",
            data_type.bright_green().bold(),
            datasets_of_type.len()
        );

        for dataset in datasets_of_type {
            println!("  📋 {}", dataset.display_name.bright_white());

            if detailed {
                println!("    🆔 ID: {}", dataset.id.italic());
                if let Some(url) = &dataset.download_entry_url {
                    println!("    🔗 URL: {}", url.dimmed());
                }
                println!(
                    "    🕒 Created: {}",
                    dataset
                        .created_at
                        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "Unknown".to_string())
                        .dimmed()
                );
            }
        }
        println!();
    }

    Ok(())
}

async fn handle_list_files(
    cache: &CacheManager,
    release_name: &str,
    detailed: bool,
    json: bool,
) -> Result<()> {
    let releases = cache.get_releases(Some(release_name)).await?;

    if releases.is_empty() {
        if json {
            return print_json(&serde_json::json!({
                "releases": [],
                "release_hint": release_name,
                "detailed": detailed
            }));
        }
        println!(
            "{}",
            format!("📁 Files for release: {}", release_name)
                .bright_cyan()
                .bold()
        );
        println!("{}", "─".repeat(50).dimmed());
        println!(
            "{} No releases found matching '{}'",
            "⚠️".yellow(),
            release_name
        );
        return Ok(());
    }

    if json {
        let releases_json: Vec<serde_json::Value> = releases
            .into_iter()
            .map(|r| {
                let files: Vec<FileSummary> = r
                    .files
                    .iter()
                    .map(|f| FileSummary {
                        filename: f.filename.clone(),
                        url: f.url.clone(),
                        md5_hash: f.md5_hash.clone(),
                        size: f.size,
                        data_type: f.data_type.clone(),
                        release_id: f.release_id.clone(),
                        is_downloaded: f.is_downloaded,
                        download_path: f.download_path.clone(),
                    })
                    .collect();

                serde_json::json!({
                    "release": ReleaseSummary {
                        id: r.id.clone(),
                        name: r.name.clone(),
                        release_date: r.release_date.as_ref().map(|d| d.to_rfc3339()),
                        is_current: r.is_current,
                        files_count: r.files.len(),
                    },
                    "files": files,
                    "detailed": detailed
                })
            })
            .collect();
        return print_json(&serde_json::json!({
            "releases": releases_json,
            "release_hint": release_name,
            "detailed": detailed
        }));
    }

    println!(
        "{}",
        format!("📁 Files for release: {}", release_name)
            .bright_cyan()
            .bold()
    );
    println!("{}", "─".repeat(50).dimmed());

    for release in releases {
        let date_str = release
            .release_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        println!(
            "📦 Release: {} ({})",
            release.name.bright_white(),
            date_str.dimmed()
        );

        if detailed {
            println!("  📁 Files ({} total):", release.files.len());
        }

        // Group files by data type
        let mut files_by_type = std::collections::HashMap::new();
        for file in &release.files {
            let data_type = file.data_type.as_deref().unwrap_or("Unknown");
            files_by_type
                .entry(data_type.to_string())
                .or_insert_with(Vec::new)
                .push(file);
        }

        let mut types: Vec<_> = files_by_type.keys().collect();
        types.sort();

        for data_type in types {
            let files_of_type = &files_by_type[data_type];

            println!(
                "\n  {} {} ({} files)",
                if detailed { "🔬" } else { "  🔬" },
                data_type.bright_green(),
                files_of_type.len()
            );

            for file in files_of_type {
                if detailed {
                    let status = if file.is_downloaded { "✅" } else { "⬜" };
                    let size_mb = file
                        .size
                        .map(|s| format!("{} MB", s / (1024 * 1024)))
                        .unwrap_or_else(|| "Unknown size".to_string());

                    println!(
                        "    {} {} ({})",
                        status,
                        file.filename.bright_white(),
                        size_mb.dimmed()
                    );
                    println!("      🆔 ID: {}", file.id.unwrap_or(0));
                    println!(
                        "      🔗 URL: {}",
                        file.url.chars().take(80).collect::<String>() + "..."
                    );

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

#[allow(clippy::too_many_arguments)]
async fn handle_download(
    cache: &CacheManager,
    command: Option<DownloadCommands>,
    dataset: Option<String>,
    file: Option<String>,
    output: String,
    workers: usize,
    skip_existing: bool,
    verify_checksum: bool,
    refresh_links: bool,
    strict_refresh: bool,
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
        Some(DownloadCommands::Manifest {
            file,
            required_only,
            retry,
        }) => {
            return handle_download_manifest(
                cache,
                &file,
                output,
                workers,
                skip_existing,
                verify_checksum,
                refresh_links,
                strict_refresh,
                required_only,
                retry,
            )
            .await;
        }
        Some(DownloadCommands::ReconcileLocal {
            from,
            to,
            manifest,
            overwrite_nonzero,
            verify_size_nonzero,
        }) => {
            return handle_reconcile_local_downloads(
                cache,
                &from,
                &to,
                &manifest,
                overwrite_nonzero,
                verify_size_nonzero,
            )
            .await;
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

    let download_items = if refresh_links {
        println!("🔄 Refreshing file URLs from DepMap API...");
        cache
            .refresh_files_from_api_by_filename(&download_items, strict_refresh)
            .await?
    } else {
        download_items
    };

    println!("Found {} files to download", download_items.len());

    let output_dir = PathBuf::from(&output);
    let downloader = Downloader::new(workers, output, skip_existing, verify_checksum)?;
    downloader.download_files(download_items.clone()).await?;

    let mut downloaded_files: Vec<(String, String)> = Vec::new();
    for item in &download_items {
        let file_path = output_dir.join(&item.filename);
        if file_path.exists() {
            downloaded_files.push((
                item.filename.clone(),
                file_path.to_string_lossy().to_string(),
            ));
        }
    }
    let updated_rows = cache.mark_files_as_downloaded(&downloaded_files).await?;
    println!(
        "🗂️  Updated download status for {} file records",
        updated_rows.to_string().bright_blue()
    );

    println!("{}", "✅ Download completed!".bright_green());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_download_manifest(
    cache: &CacheManager,
    manifest_file: &str,
    output: String,
    workers: usize,
    skip_existing: bool,
    verify_checksum: bool,
    refresh_links: bool,
    strict_refresh: bool,
    required_only: bool,
    retry: usize,
) -> Result<()> {
    println!(
        "📄 Loading manifest: {}",
        PathBuf::from(manifest_file)
            .display()
            .to_string()
            .bright_cyan()
    );

    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .from_path(manifest_file)?;

    let mut rows: Vec<ManifestRow> = Vec::new();
    for row in reader.deserialize() {
        let parsed: ManifestRow = row?;
        if parsed.filename.trim().is_empty() || parsed.filename == "(unresolved_from_cache)" {
            continue;
        }
        if required_only && !flag_is_yes(parsed.required_for_alignment.as_deref()) {
            continue;
        }
        rows.push(parsed);
    }

    if rows.is_empty() {
        println!("{} No manifest rows selected", "⚠️".yellow());
        return Ok(());
    }

    let mut dataset_cache: HashMap<String, HashMap<String, DownloadFile>> = HashMap::new();
    let mut release_cache: HashMap<String, HashMap<String, DownloadFile>> = HashMap::new();
    let mut selected_files: Vec<DownloadFile> = Vec::new();
    let mut dedup = HashSet::new();

    for row in rows {
        let key = format!(
            "{}::{}::{}",
            row.dataset_id.clone().unwrap_or_default(),
            row.release_id.clone().unwrap_or_default(),
            row.filename
        );
        if !dedup.insert(key) {
            continue;
        }

        let mut resolved: Option<DownloadFile> = None;

        if let Some(dataset_id) = row.dataset_id.clone() {
            let entry = dataset_cache.entry(dataset_id.clone()).or_default();
            if entry.is_empty() {
                match cache.get_dataset_files(&dataset_id).await {
                    Ok(files) => {
                        for f in files {
                            entry.insert(f.filename.clone(), f);
                        }
                    }
                    Err(e) => warn!("Dataset '{}' lookup failed: {}", dataset_id, e),
                }
            }
            resolved = entry.get(&row.filename).cloned();
        }

        if resolved.is_none() {
            if let Some(release_id) = row.release_id.clone() {
                let entry = release_cache.entry(release_id.clone()).or_default();
                if entry.is_empty() {
                    match cache.get_release_files(&release_id, None).await {
                        Ok(files) => {
                            for f in files {
                                entry.insert(f.filename.clone(), f);
                            }
                        }
                        Err(e) => warn!("Release '{}' lookup failed: {}", release_id, e),
                    }
                }
                resolved = entry.get(&row.filename).cloned();
            }
        }

        if resolved.is_none() {
            resolved = cache.get_file_by_name(&row.filename).await.ok();
        }

        if let Some(file) = resolved {
            selected_files.push(file);
        } else {
            warn!("Manifest file '{}' not found in cache", row.filename);
        }
    }

    if selected_files.is_empty() {
        return Err(DepMapError::NotFound(
            "No downloadable files were resolved from manifest".to_string(),
        ));
    }

    let output_dir = PathBuf::from(&output);
    let downloader = Downloader::new(workers, output.clone(), skip_existing, verify_checksum)?;
    let mut remaining = if refresh_links {
        println!("🔄 Refreshing manifest file URLs from DepMap API...");
        cache
            .refresh_files_from_api_by_filename(&selected_files, strict_refresh)
            .await?
    } else {
        selected_files
    };
    let mut attempts = 0usize;

    loop {
        attempts += 1;
        println!(
            "📥 Manifest download attempt {}/{} ({} files)",
            attempts,
            retry + 1,
            remaining.len()
        );
        downloader.download_files(remaining.clone()).await?;

        let mut downloaded_files: Vec<(String, String)> = Vec::new();
        let mut next_remaining: Vec<DownloadFile> = Vec::new();
        for item in &remaining {
            let file_path = output_dir.join(&item.filename);
            if file_path.exists() {
                downloaded_files.push((
                    item.filename.clone(),
                    file_path.to_string_lossy().to_string(),
                ));
            } else {
                next_remaining.push(item.clone());
            }
        }

        if !downloaded_files.is_empty() {
            let updated_rows = cache.mark_files_as_downloaded(&downloaded_files).await?;
            println!(
                "🗂️  Updated download status for {} file records",
                updated_rows.to_string().bright_blue()
            );
        }

        if next_remaining.is_empty() {
            println!("{}", "✅ Manifest download completed!".bright_green());
            return Ok(());
        }
        if attempts > retry {
            return Err(DepMapError::DownloadError(format!(
                "{} files still missing after {} attempts",
                next_remaining.len(),
                attempts
            )));
        }

        remaining = if refresh_links {
            cache
                .refresh_files_from_api_by_filename(&next_remaining, strict_refresh)
                .await?
        } else {
            next_remaining
        };
        warn!(
            "{} files missing after attempt {}, retrying...",
            remaining.len(),
            attempts
        );
    }
}

async fn handle_reconcile_local_downloads(
    cache: &CacheManager,
    from_dir: &str,
    to_dir: &str,
    manifest_file: &str,
    overwrite_nonzero: bool,
    verify_size_nonzero: bool,
) -> Result<()> {
    const CTRP_CONFLICT_FILE: &str = "CTRPv2.0_2015_ctd2_ExpandedDataset.zip";

    println!(
        "🧩 Reconciling local downloads from {} -> {}",
        from_dir.bright_cyan(),
        to_dir.bright_cyan()
    );
    println!("📄 Using manifest: {}", manifest_file.bright_cyan());

    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .from_path(manifest_file)?;
    let mut filenames = HashSet::new();
    for row in reader.deserialize() {
        let parsed: ManifestRow = row?;
        if !parsed.filename.trim().is_empty() {
            filenames.insert(parsed.filename);
        }
    }

    if filenames.is_empty() {
        return Err(DepMapError::NotFound(
            "No filenames found in manifest".to_string(),
        ));
    }

    let from = Path::new(from_dir);
    let to = Path::new(to_dir);
    fs::create_dir_all(to).await?;

    let mut copied = 0usize;
    let mut skipped_nonzero = 0usize;
    let mut skipped_conflict = 0usize;
    let mut missing_sources = 0usize;
    let mut invalid_sources = 0usize;
    let mut downloaded_files: Vec<(String, String)> = Vec::new();

    let mut sorted_files: Vec<String> = filenames.into_iter().collect();
    sorted_files.sort();

    for filename in sorted_files {
        let src = from.join(&filename);
        let dst = to.join(&filename);

        if !src.exists() {
            warn!("Source file missing: {}", src.display());
            missing_sources += 1;
            continue;
        }

        let src_size = fs::metadata(&src).await?.len();
        if verify_size_nonzero && src_size == 0 {
            warn!("Source file is zero-byte, skip: {}", src.display());
            invalid_sources += 1;
            continue;
        }

        let dst_exists = dst.exists();
        let dst_size = if dst_exists {
            Some(fs::metadata(&dst).await?.len())
        } else {
            None
        };

        if filename == CTRP_CONFLICT_FILE {
            if let Some(existing) = dst_size {
                if existing > 0 && existing != src_size {
                    warn!(
                        "Skipping conflict file {} (target size {} != source size {})",
                        filename, existing, src_size
                    );
                    skipped_conflict += 1;
                    continue;
                }
            }
        }

        if dst_exists && !overwrite_nonzero {
            if let Some(existing) = dst_size {
                if existing > 0 {
                    skipped_nonzero += 1;
                    continue;
                }
            }
        }

        fs::copy(&src, &dst).await?;
        copied += 1;
        downloaded_files.push((filename.clone(), dst.to_string_lossy().to_string()));
    }

    let updated_rows = if downloaded_files.is_empty() {
        0
    } else {
        cache.mark_files_as_downloaded(&downloaded_files).await?
    };

    println!("\n📊 Reconcile Summary:");
    println!("✅ Copied: {}", copied.to_string().bright_green());
    println!("🗂️  DB rows marked downloaded: {}", updated_rows);
    println!("⏭️  Skipped existing non-zero: {}", skipped_nonzero);
    println!("⚠️  Skipped conflict (CTRP): {}", skipped_conflict);
    println!("⚠️  Missing source files: {}", missing_sources);
    println!("⚠️  Invalid source files: {}", invalid_sources);

    if copied == 0 {
        return Err(DepMapError::DownloadError(
            "No files were copied during reconcile-local".to_string(),
        ));
    }

    Ok(())
}

async fn handle_align(command: AlignCommands) -> Result<()> {
    match command {
        AlignCommands::BuildIndex => {
            println!(
                "{}",
                "🧱 Building cellline+intervention index...".bright_blue()
            );
            run_script("python3", &["scripts/build_cellline_intervention_index.py"])?;
            println!("{}", "✅ Index build complete".bright_green());
        }
        AlignCommands::BuildLongTable { assays } => {
            println!("{}", "🧬 Building multimodal long table...".bright_blue());
            let mut args = vec!["scripts/build_multimodal_long_table.py"];
            if let Some(list) = assays.as_deref() {
                args.push("--assays");
                args.push(list);
            }
            run_script("python3", &args)?;
            println!("{}", "✅ Long-table build complete".bright_green());
        }
    }
    Ok(())
}

fn run_script(binary: &str, args: &[&str]) -> Result<()> {
    let mut owned_args: Vec<String> = Vec::new();
    for (idx, arg) in args.iter().enumerate() {
        if idx == 0 {
            owned_args.push(resolve_script_path(arg).to_string_lossy().to_string());
        } else {
            owned_args.push((*arg).to_string());
        }
    }

    let status = Command::new(binary).args(&owned_args).status()?;
    if !status.success() {
        return Err(DepMapError::DownloadError(format!(
            "command failed: {} {}",
            binary,
            owned_args.join(" ")
        )));
    }
    Ok(())
}

fn resolve_script_path(path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.exists() {
        return candidate;
    }
    let parent_candidate = PathBuf::from("..").join(path);
    if parent_candidate.exists() {
        return parent_candidate;
    }
    PathBuf::from(path)
}

async fn handle_search(
    cache: &CacheManager,
    query: &str,
    cell_line: bool,
    gene: bool,
    dataset: bool,
    limit: Option<usize>,
    json: bool,
) -> Result<()> {
    let limit = limit.unwrap_or(50);
    let search_all = !cell_line && !gene && !dataset;

    if json {
        let genes = if gene || search_all {
            Some(cache.search_genes(query, limit).await?)
        } else {
            None
        };
        let cell_lines = if cell_line || search_all {
            Some(cache.search_cell_lines(query).await?)
        } else {
            None
        };
        let datasets = if dataset || search_all {
            Some(cache.search_datasets(query).await?)
        } else {
            None
        };

        return print_json(&serde_json::json!({
            "query": query,
            "limit": limit,
            "genes": genes,
            "cell_lines": cell_lines,
            "datasets": datasets
        }));
    }

    println!(
        "{}\n",
        format!("🔍 Searching for: {}", query.bright_cyan()).bold()
    );

    if gene || search_all {
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

                        println!(
                            "    🧬 {} (Entrez ID: {})",
                            gene.gene.bright_white(),
                            gene.entrez_id.to_string().dimmed()
                        );
                        println!("      📊 Dataset: {}", gene.dataset.italic());
                        println!(
                            "      📈 Dependent Cell Lines: {}",
                            gene.dependent_cell_lines
                        );
                        println!(
                            "      🧪 Cell Lines with Data: {}",
                            gene.cell_lines_with_data
                        );
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

    if cell_line || search_all {
        println!("🧬 Searching in cell lines...");
        match cache.search_cell_lines(query).await {
            Ok(cell_lines) => {
                if cell_lines.is_empty() {
                    println!("  No cell lines found");
                } else {
                    let limited: Vec<_> = cell_lines.into_iter().take(limit).collect();
                    println!("  Found {} matching cell lines:", limited.len());
                    for cell_line in limited {
                        println!(
                            "    🧬 {} ({})",
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

    if dataset || search_all {
        println!("📊 Searching in datasets...");
        match cache.search_datasets(query).await {
            Ok(datasets) => {
                if datasets.is_empty() {
                    println!("  No datasets found");
                } else {
                    let limited: Vec<_> = datasets.into_iter().take(limit).collect();
                    println!("  Found {} matching datasets:", limited.len());
                    for dataset in limited {
                        println!(
                            "    📊 {} ({})",
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

async fn handle_stats(cache: &CacheManager, detailed: bool, json: bool) -> Result<()> {
    let stats = cache.get_cache_stats().await?;
    let progress_dir = resolve_depmap_data_dir();
    let progress = cache.get_download_progress(&progress_dir).await?;

    if json {
        return print_json(&serde_json::json!({
            "stats": stats,
            "progress": progress,
            "progress_dir": progress_dir,
            "detailed": detailed
        }));
    }

    println!("{}\n", "📊 Cache Statistics".bright_cyan().bold());

    println!(
        "📦 Releases: {}",
        stats.release_count.to_string().bright_green()
    );
    println!("📁 Files: {}", stats.file_count.to_string().bright_green());
    println!(
        "📊 Datasets: {}",
        stats.dataset_count.to_string().bright_green()
    );
    println!(
        "🧬 Cell Lines: {}",
        stats.cell_line_count.to_string().bright_green()
    );
    println!(
        "🧪 Gene Dependencies: {}",
        stats.gene_dependency_count.to_string().bright_green()
    );

    if stats.total_size_mb > 0 {
        println!(
            "💾 Total Size: {} MB",
            stats.total_size_mb.to_string().bright_yellow()
        );
    }

    if let Some(last_updated) = stats.last_updated {
        println!(
            "🕒 Last Updated: {}",
            last_updated
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string()
                .bright_blue()
        );
    } else {
        println!("🕒 Last Updated: {}", "Never".bright_red());
    }

    if progress.total_files > 0 {
        let pct = (progress.present_files as f64 / progress.total_files as f64) * 100.0;
        println!(
            "📥 On Disk (by filename): {}/{} ({:.1}%)",
            progress.present_files.to_string().bright_green(),
            progress.total_files.to_string().bright_green(),
            pct
        );
    } else {
        println!("📥 On Disk (by filename): {}", "No tracked files".yellow());
    }
    println!(
        "🗃️ DB-marked downloaded files: {}",
        progress
            .db_marked_downloaded_files
            .to_string()
            .bright_green()
    );

    if detailed {
        println!("\n🔍 Detailed Information:");

        // Show release breakdown
        let releases = cache.get_releases(None).await?;
        println!("\n📦 Releases:");
        for release in releases {
            let marker = if release.is_current { " 🌟" } else { "" };
            println!(
                "  {}{} ({} files)",
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
            println!(
                "  {}: {}",
                data_type.italic(),
                count.to_string().bright_green()
            );
        }

        println!("\n📥 Download Progress by Data Type:");
        for item in &progress.by_data_type {
            let pct = if item.total_files > 0 {
                (item.present_files as f64 / item.total_files as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "  {}: {}/{} ({:.1}%)",
                item.category.italic(),
                item.present_files.to_string().bright_green(),
                item.total_files.to_string().bright_green(),
                pct
            );
        }

        println!("\n📦 Download Progress by Release:");
        for item in &progress.by_release {
            let pct = if item.total_files > 0 {
                (item.present_files as f64 / item.total_files as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "  {}: {}/{} ({:.1}%)",
                item.category.bold(),
                item.present_files.to_string().bright_green(),
                item.total_files.to_string().bright_green(),
                pct
            );
        }
    }

    Ok(())
}

async fn handle_clear(cache: &CacheManager, all: bool, data_type: Option<String>) -> Result<()> {
    if all {
        println!("{} Clearing all cached data...", "🗑️".bright_red());
        cache.clear_all_cache().await?;
        println!(
            "{}",
            "✅ All cached data cleared successfully!".bright_green()
        );
    } else if let Some(d_type) = data_type {
        println!(
            "{} Clearing cached data of type: {}",
            "🗑️".bright_red(),
            d_type
        );
        let deleted_count = cache.clear_cache_by_data_type(&d_type).await?;
        println!(
            "{}",
            format!("✅ Cleared {} files of type '{}'!", deleted_count, d_type).bright_green()
        );
    } else {
        println!(
            "{}",
            "❌ No clear option specified. Use --all or --data-type".red()
        );
    }

    Ok(())
}

fn resolve_depmap_data_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("depmap_data"),
        PathBuf::from("../depmap_data"),
    ];
    for candidate in candidates {
        if candidate.exists() && candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from("depmap_data")
}

#[cfg(test)]
mod tests {
    use super::flag_is_yes;

    #[test]
    fn parses_yes_flags() {
        assert!(!flag_is_yes(None));
        assert!(!flag_is_yes(Some("")));
        assert!(!flag_is_yes(Some("n")));
        assert!(flag_is_yes(Some("Y")));
        assert!(flag_is_yes(Some(" yes ")));
        assert!(flag_is_yes(Some("TRUE")));
        assert!(flag_is_yes(Some("1")));
    }
}
