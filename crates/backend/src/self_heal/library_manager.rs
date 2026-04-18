use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use futures::StreamExt;
use crate::directories::LauncherDirectories;

#[derive(Error, Debug)]
pub enum LibraryManagerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Library not found: {0}")]
    NotFound(String),
    #[error("Hash mismatch for: {0}")]
    HashMismatch(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Failed to download: {0}")]
    DownloadFailed(Arc<str>),
    #[error("Invalid library data")]
    InvalidData,
}

#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub name: String,
    pub path: PathBuf,
    pub expected_hash: [u8; 20],
    pub size: u64,
}

pub struct LibraryManager {
    http_client: reqwest::Client,
    libraries_dir: Arc<Path>,
}

impl LibraryManager {
    pub fn new(http_client: reqwest::Client, directories: Arc<LauncherDirectories>) -> Self {
        Self {
            http_client,
            libraries_dir: directories.libraries_dir.clone(),
        }
    }

    pub fn get_library_path(&self, name: &str, expected_hash: [u8; 20]) -> PathBuf {
        crate::create_content_library_path(&self.libraries_dir, expected_hash, None)
    }

    pub async fn validate_library(&self, path: &Path, expected_hash: [u8; 20]) -> Result<bool, LibraryManagerError> {
        if !path.exists() {
            return Ok(false);
        }

        let valid = crate::check_sha1_hash(path, expected_hash)?;
        Ok(valid)
    }

    pub async fn ensure_library(
        &self,
        name: &str,
        url: &str,
        expected_hash: [u8; 20],
        on_progress: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
    ) -> Result<PathBuf, LibraryManagerError> {
        let dest_path = self.get_library_path(name, expected_hash);

        if dest_path.exists() {
            if self.validate_library(&dest_path, expected_hash).await? {
                return Ok(dest_path);
            }
            std::fs::remove_file(&dest_path)?;
        }

        self.download_library(url, &dest_path, on_progress).await?;

        if !self.validate_library(&dest_path, expected_hash).await? {
            std::fs::remove_file(&dest_path)?;
            return Err(LibraryManagerError::HashMismatch(name.to_string()));
        }

        Ok(dest_path)
    }

    async fn download_library(
        &self,
        url: &str,
        dest: &Path,
        on_progress: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
    ) -> Result<(), LibraryManagerError> {
        let response = self.http_client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(LibraryManagerError::DownloadFailed(Arc::from(format!("HTTP {}", response.status()))));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0usize;

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(dest)?;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            downloaded += chunk.len();

            if let Some(progress_cb) = &on_progress {
                progress_cb(downloaded, total_size as usize);
            }
        }

        Ok(())
    }

    pub async fn repair_library(
        &self,
        path: &Path,
        url: &str,
        expected_hash: [u8; 20],
    ) -> Result<PathBuf, LibraryManagerError> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;

        self.download_library(url, path, None).await?;

        if !self.validate_library(path, expected_hash).await? {
            std::fs::remove_file(path)?;
            return Err(LibraryManagerError::HashMismatch(path.display().to_string()));
        }

        Ok(path.to_path_buf())
    }

    pub fn cleanup_orphaned_libraries(&self, valid_hashes: &[[u8; 20]]) {
        let libraries_dir = &self.libraries_dir;

        if !libraries_dir.exists() {
            return;
        }

        let Ok(entries) = std::fs::read_dir(libraries_dir) else {
            return;
        };

        let valid_set: std::collections::HashSet<[u8; 20]> = valid_hashes.iter().copied().collect();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(Ok(hash)) = crate::self_heal::utils::compute_sha1(&path).map(|h| {
                    let mut s = [0u8; 20];
                    s.copy_from_slice(&h);
                    Ok::<[u8; 20], ()>(s)
                }) {
                    if !valid_set.contains(&hash) {
                        log::debug!("Removing orphaned library: {:?}", path);
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
    }
}
