use crate::error::{DepMapError, Result};
use crate::models::DownloadFile;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::sync::Semaphore;
use tracing::{info, warn, error, debug};
use futures::future::BoxFuture;

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
        
        println!("📥 Starting download of {} files with {} workers", 
            total_files.to_string().bright_green(), 
            self.max_workers.to_string().bright_blue()
        );
        
        let mut download_tasks = Vec::new();
        
        for file in files {
            let file_path = self.output_dir.join(&file.filename);
            
            // Skip existing files if requested
            if self.skip_existing && file_path.exists() {
                println!("⏭️  Skipping existing file: {}", file.filename.bright_green());
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
                    overall_progress
                ).await
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
        println!("✅ Successful: {}", successful_downloads.to_string().bright_green());
        if failed_downloads > 0 {
            println!("❌ Failed: {}", failed_downloads.to_string().bright_red());
        }
        println!("📁 Output directory: {}", self.output_dir.display().to_string().bright_blue());
        
        Ok(())
    }
    
    async fn download_single_file(
        &self,
        client: reqwest::Client,
        file: DownloadFile,
        output_dir: PathBuf,
        verify_checksum: bool,
        overall_progress: ProgressBar,
    ) -> Result<()> {
        let file_path = output_dir.join(&file.filename);
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        println!("📥 Downloading: {}", file.filename.bright_cyan());
        
        // Start download
        let response = client.get(&file.url).send().await?;
        
        if !response.status().is_success() {
            return Err(DepMapError::DownloadError(
                format!("HTTP {}: {}", response.status(), file.url)
            ));
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
        
        // Download and write file
        let mut downloaded_bytes = 0u64;
        let mut hasher = Sha1::new();
        
        let mut file_handle = File::create(&file_path).await?;
        let bytes = response.bytes().await?;
        
        // Update hash
        if verify_checksum {
            hasher.update(&bytes);
        }
        
        // Write to file
        file_handle.write_all(&bytes).await?;
        
        downloaded_bytes = bytes.len() as u64;
        
        // Update progress bars
        if let Some(ref progress) = file_progress {
            progress.set_position(downloaded_bytes);
        }
        
        file_handle.flush().await?;
        
        if let Some(ref progress) = file_progress {
            progress.finish();
        }
        
        // Verify checksum if requested
        if verify_checksum {
            if let Some(expected_hash) = &file.md5_hash {
                let calculated_hash = format!("{:x}", hasher.finalize());
                if calculated_hash != *expected_hash {
                    // Remove corrupted file
                    tokio::fs::remove_file(&file_path).await?;
                    return Err(DepMapError::ChecksumError(format!(
                        "Checksum mismatch for {}: expected {}, got {}",
                        file.filename, expected_hash, calculated_hash
                    )));
                }
                println!("  ✅ Checksum verified");
            }
        }
        
        println!("  ✅ Complete ({} bytes)", downloaded_bytes.to_string().bright_green());
        overall_progress.inc(1);
        
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
        let file_path = output_dir.join(&file.filename);
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        println!("📥 Downloading: {}", file.filename.bright_cyan());
        
        // Start download
        let response = client.get(&file.url).send().await?;
        
        if !response.status().is_success() {
            return Err(DepMapError::DownloadError(
                format!("HTTP {}: {}", response.status(), file.url)
            ));
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
        
        // Download and write file
        let bytes = response.bytes().await?;
        
        // Update hash if needed
        let mut hasher = Sha1::new();
        if verify_checksum {
            hasher.update(&bytes);
        }
        
        // Write to file
        let mut file_handle = File::create(&file_path).await?;
        file_handle.write_all(&bytes).await?;
        
        let downloaded_bytes = bytes.len() as u64;
        
        // Update progress bars
        if let Some(ref progress) = file_progress {
            progress.set_position(downloaded_bytes);
            progress.finish();
        }
        
        file_handle.flush().await?;
        
        // Verify checksum if requested
        if verify_checksum {
            if let Some(expected_hash) = &file.md5_hash {
                let calculated_hash = format!("{:x}", hasher.finalize());
                if calculated_hash != *expected_hash {
                    // Remove corrupted file
                    tokio::fs::remove_file(&file_path).await?;
                    return Err(DepMapError::ChecksumError(format!(
                        "Checksum mismatch for {}: expected {}, got {}",
                        file.filename, expected_hash, calculated_hash
                    )));
                }
                println!("  ✅ Checksum verified");
            }
        }
        
        println!("  ✅ Complete ({} bytes)", downloaded_bytes.to_string().bright_green());
        overall_progress.inc(1);
        
        Ok(())
    }
    
    pub async fn get_file_info(&self, file: &DownloadFile) -> Result<FileDownloadInfo> {
        let file_path = self.output_dir.join(&file.filename);
        
        let exists = file_path.exists();
        let size = if exists {
            Some(tokio::fs::metadata(&file_path).await?.len())
        } else {
            None
        };
        
        let checksum = if exists && self.verify_checksum {
            self.calculate_file_checksum(&file_path).await?
        } else {
            None
        };
        
        Ok(FileDownloadInfo {
            filename: file.filename.clone(),
            exists,
            size,
            expected_size: file.size,
            checksum,
            expected_checksum: file.md5_hash.clone(),
        })
    }
    
    async fn calculate_file_checksum(&self, file_path: &Path) -> Result<Option<String>> {
        let mut file = File::open(file_path).await?;
        let mut hasher = Sha1::new();
        let mut buffer = [0; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(Some(format!("{:x}", hasher.finalize())))
    }
}

#[derive(Debug)]
pub struct FileDownloadInfo {
    pub filename: String,
    pub exists: bool,
    pub size: Option<u64>,
    pub expected_size: Option<u64>,
    pub checksum: Option<String>,
    pub expected_checksum: Option<String>,
}

impl FileDownloadInfo {
    pub fn is_complete(&self) -> bool {
        if !self.exists {
            return false;
        }
        
        // Check size if expected size is known
        if let (Some(actual), Some(expected)) = (self.size, self.expected_size) {
            if actual != expected {
                return false;
            }
        }
        
        // Check checksum if expected checksum is known
        if let (Some(actual), Some(expected)) = (&self.checksum, &self.expected_checksum) {
            if actual != expected {
                return false;
            }
        }
        
        true
    }
}
