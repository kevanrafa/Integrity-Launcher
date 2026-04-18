use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use rc_zip_sync::ReadZip;

#[derive(Error, Debug)]
pub enum NativesManagerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid archive: {0}")]
    InvalidArchive(String),
    #[error("Failed to extract: {0}")]
    ExtractionFailed(String),
    #[error("Hash mismatch")]
    HashMismatch,
    #[error("Unsupported platform")]
    UnsupportedPlatform,
    #[error("Zip error: {0}")]
    ZipError(#[from] rc_zip_sync::rc_zip::Error),
}

pub struct NativesManager {
    natives_base_dir: PathBuf,
}

impl NativesManager {
    pub fn new(directories: &crate::directories::LauncherDirectories) -> Self {
        Self {
            natives_base_dir: directories.temp_natives_base_dir.to_path_buf(),
        }
    }

    pub fn get_extraction_dir(&self, instance_id: &str) -> PathBuf {
        self.natives_base_dir.join(instance_id)
    }

    pub async fn extract_natives(
        &self,
        instance_id: &str,
        archive_path: &Path,
        natives: &[String],
    ) -> Result<Vec<PathBuf>, NativesManagerError> {
        let extract_dir = self.get_extraction_dir(instance_id);
        std::fs::create_dir_all(&extract_dir)?;

        self.clear_old(&extract_dir)?;
        self.extract_archive(archive_path, &extract_dir, natives).await
    }

    fn clear_old(&self, dir: &Path) -> std::io::Result<()> {
        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() || (path.is_dir() && entry.file_name() != ".lock") {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        Ok(())
    }

    async fn extract_archive(
        &self,
        archive: &Path,
        dest: &Path,
        natives: &[String],
    ) -> Result<Vec<PathBuf>, NativesManagerError> {
        let file = std::fs::File::open(archive)?;
        let is_zip = archive.extension().map_or(false, |ext| ext == "zip" || ext == "jar");

        if !is_zip {
            return Err(NativesManagerError::InvalidArchive("Only zip/jar archives supported".into()));
        }

        let zip_archive = file.read_zip()?;
        let mut extracted = Vec::new();

        for entry in zip_archive.entries() {
            let name = entry.name.to_string();

            if should_extract_native(&name, natives) {
                let out_path = dest.join(&name);

                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                match entry.kind() {
                    rc_zip_sync::rc_zip::EntryKind::Directory => {
                        std::fs::create_dir_all(&out_path)?;
                    }
                    rc_zip_sync::rc_zip::EntryKind::File => {
                        let buffer = entry.bytes()?;
                        std::fs::write(&out_path, buffer)?;
                    }
                    _ => {}
                }

                extracted.push(out_path);
            }
        }

        Ok(extracted)
    }

    pub fn get_native_lib_path(&self, instance_id: &str, lib_name: &str) -> PathBuf {
        self.get_extraction_dir(instance_id).join(lib_name)
    }

    pub fn cleanup_instance_natives(&self, instance_id: &str) {
        let dir = self.get_extraction_dir(instance_id);
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

fn should_extract_native(name: &str, natives: &[String]) -> bool {
    if natives.is_empty() {
        return true;
    }

    let lower_name = name.to_lowercase();
    natives.iter().any(|native| {
        let native_lower = native.to_lowercase();
        lower_name.contains(&native_lower) || 
        lower_name.ends_with(&format!("lib{}.so", native_lower)) ||
        (cfg!(windows) && lower_name.ends_with(&format!("{}.dll", native_lower))) ||
        (cfg!(target_os = "macos") && lower_name.ends_with(&format!("lib{}.dylib", native_lower)))
    })
}
