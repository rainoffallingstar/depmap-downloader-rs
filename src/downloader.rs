use crate::error::{DepMapError, Result};
use crate::models::DownloadFile;
use colored::*;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tracing::error;

pub struct Downloader {
    max_workers: usize,
    output_dir: PathBuf,
    client: reqwest::Client,
    skip_existing: bool,
    verify_checksum: bool,
}

impl Downloader {
    pub fn new(
        max_workers: usize,
        output_dir: String,
        skip_existing: bool,
        verify_checksum: bool,
    ) -> Result<Self> {
        let output_path = PathBuf::from(output_dir);

        // Create output directory if it doesn't exist
        std::fs::create_dir_all(&output_path)?;

        let client = reqwest::Client::builder()
            .user_agent("depmap-downloader-rs/0.1.0")
            .build()?;

        Ok(Downloader {
            max_workers,
            output_dir: output_path,
            client,
            skip_existing,
            verify_checksum,
        })
    }

    pub async fn download_files(&self, files: Vec<DownloadFile>) -> Result<()> {
        if files.is_empty() {
            println!("{} No files to download", "⚠️".yellow());
            return Ok(());
        }

        let semaphore = Arc::new(Semaphore::new(self.max_workers));
        let total_files = files.len();

        // Create overall progress bar
        let overall_progress = ProgressBar::new(total_files as u64);
        overall_progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
                .expect("Failed to set progress bar template")
                .progress_chars("#>-")
        );

        println!(
            "📥 Starting download of {} files with {} workers",
            total_files.to_string().bright_green(),
            self.max_workers.to_string().bright_blue()
        );

        let mut download_tasks = Vec::new();

        for file in files {
            let file_path = self.output_dir.join(&file.filename);

            // Skip existing files if requested
            if self.skip_existing && file_path.exists() {
                println!(
                    "⏭️  Skipping existing file: {}",
                    file.filename.bright_green()
                );
                overall_progress.inc(1);
                continue;
            }

            let semaphore = Arc::clone(&semaphore);
            let overall_progress = overall_progress.clone();
            let client = self.client.clone();
            let output_dir = self.output_dir.clone();
            let verify_checksum = self.verify_checksum;

            download_tasks.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await?;

                Self::download_single_file_static(
                    client,
                    file,
                    output_dir,
                    verify_checksum,
                    overall_progress,
                )
                .await
            }));
        }

        // Wait for all downloads to complete
        let mut successful_downloads = 0;
        let mut failed_downloads = 0;

        for task in download_tasks {
            match task.await {
                Ok(Ok(())) => successful_downloads += 1,
                Ok(Err(e)) => {
                    error!("Download failed: {}", e);
                    failed_downloads += 1;
                }
                Err(e) => {
                    error!("Task failed: {}", e);
                    failed_downloads += 1;
                }
            }
        }

        overall_progress.finish();

        println!("\n📊 Download Summary:");
        println!(
            "✅ Successful: {}",
            successful_downloads.to_string().bright_green()
        );
        if failed_downloads > 0 {
            println!("❌ Failed: {}", failed_downloads.to_string().bright_red());
        }
        println!(
            "📁 Output directory: {}",
            self.output_dir.display().to_string().bright_blue()
        );

        Ok(())
    }

    // Static method to be used in async tasks
    async fn download_single_file_static(
        client: reqwest::Client,
        file: DownloadFile,
        output_dir: PathBuf,
        verify_checksum: bool,
        overall_progress: ProgressBar,
    ) -> Result<()> {
        struct ProgressIncOnDrop(ProgressBar);
        impl Drop for ProgressIncOnDrop {
            fn drop(&mut self) {
                self.0.inc(1);
            }
        }

        let file_path = output_dir.join(&file.filename);
        let _progress_guard = ProgressIncOnDrop(overall_progress.clone());

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        println!("📥 Downloading: {}", file.filename.bright_cyan());

        // Start download
        let response = client.get(&file.url).send().await?;

        if !response.status().is_success() {
            return Err(DepMapError::DownloadError(format!(
                "HTTP {}: {}",
                response.status(),
                file.url
            )));
        }

        let total_size = response.content_length().unwrap_or(0);

        // Create file progress bar
        let file_progress = if total_size > 0 {
            Some(ProgressBar::new(total_size))
        } else {
            None
        };

        if let Some(ref progress) = file_progress {
            progress.set_style(
                ProgressStyle::default_bar()
                    .template(&format!("  📄 {} {{spinner:.green}} [{{bar:40.blue}}] {{bytes}}/{{total_bytes}} ({{eta}})", file.filename))
                    .expect("Failed to set file progress bar template")
                    .progress_chars("#>-")
            );
        }

        let mut file_handle = File::create(&file_path).await?;
        let mut downloaded_bytes: u64 = 0;
        let mut hasher = if verify_checksum && file.md5_hash.is_some() {
            Some(md5::Context::new())
        } else {
            None
        };

        let download_result: Result<()> = async {
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                file_handle.write_all(&chunk).await?;
                downloaded_bytes += chunk.len() as u64;
                if let Some(ref mut ctx) = hasher {
                    ctx.consume(&chunk);
                }
                if let Some(ref progress) = file_progress {
                    progress.set_position(downloaded_bytes);
                }
            }
            file_handle.flush().await?;
            Ok(())
        }
        .await;

        if let Err(e) = download_result {
            let _ = tokio::fs::remove_file(&file_path).await;
            if let Some(ref progress) = file_progress {
                progress.abandon();
            }
            return Err(e);
        }

        if let Some(ref progress) = file_progress {
            progress.finish();
        }

        if verify_checksum {
            if let (Some(expected_hash), Some(ctx)) = (file.md5_hash.as_deref(), hasher) {
                let calculated_hash = format!("{:x}", ctx.compute());
                if !calculated_hash.eq_ignore_ascii_case(expected_hash) {
                    tokio::fs::remove_file(&file_path).await?;
                    return Err(DepMapError::ChecksumError(format!(
                        "Checksum mismatch for {}: expected {}, got {}",
                        file.filename, expected_hash, calculated_hash
                    )));
                }
                println!("  ✅ Checksum verified");
            }
        }

        println!(
            "  ✅ Complete ({} bytes)",
            downloaded_bytes.to_string().bright_green()
        );

        Ok(())
    }
}
