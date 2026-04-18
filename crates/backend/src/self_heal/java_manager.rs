use std::path::{Path, PathBuf};
use std::io::Write;
use std::sync::Arc;
use thiserror::Error;
use ::hex;
use crate::directories::LauncherDirectories;

#[derive(Error, Debug)]
pub enum JavaManagerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Java runtime not found: {0}")]
    NotFound(String),
    #[error("Invalid Java version: {0}")]
    InvalidVersion(String),
    #[error("Failed to download Java: {0}")]
    DownloadFailed(Arc<str>),
    #[error("Unsupported operating system: {0}")]
    UnsupportedOS(String),
    #[error("Hash verification failed")]
    HashMismatch,
    #[error("Invalid data: {0}")]
    InvalidData(Arc<str>),
}

#[derive(Debug, Clone)]
pub struct JavaLocation {
    pub path: PathBuf,
    pub version: String,
    pub arch: String,
    pub is_jre: bool,
}

pub struct JavaManager {
    http_client: reqwest::Client,
    directories: Arc<LauncherDirectories>,
}

impl JavaManager {
    pub fn new(http_client: reqwest::Client, directories: Arc<LauncherDirectories>) -> Self {
        Self { http_client, directories }
    }

    pub fn get_runtime_dir(&self) -> PathBuf {
        self.directories.runtime_base_dir.join("java")
    }

    pub async fn get_java_for_version(
        &self,
        version: &str,
        arch: &str,
        is_jre: bool,
    ) -> Result<JavaLocation, JavaManagerError> {
        let runtime_dir = self.get_runtime_dir();
        let version_dir = runtime_dir.join(format!("{}-{}{}", version, arch, if is_jre { "-jre" } else { "" }));

        if version_dir.exists() {
            let java_bin = if cfg!(windows) {
                version_dir.join("bin").join("java.exe")
            } else {
                version_dir.join("bin").join("java")
            };

            if java_bin.exists() {
                return Ok(JavaLocation {
                    path: version_dir,
                    version: version.to_string(),
                    arch: arch.to_string(),
                    is_jre,
                });
            }
        }

        Err(JavaManagerError::NotFound(format!("Java {} {} {}", version, arch, if is_jre { "JRE" } else { "JDK" })))
    }

    pub async fn download_java(
        &self,
        version: &str,
        arch: &str,
        is_jre: bool,
        on_progress: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
    ) -> Result<JavaLocation, JavaManagerError> {
        let runtime_dir = self.get_runtime_dir();
        std::fs::create_dir_all(&runtime_dir)?;

        let download_info = self.get_java_download_url(version, arch, is_jre).await?;
        let dest_path = runtime_dir.join(format!("{}-{}{}.tar.gz", version, arch, if is_jre { "-jre" } else { "" }));

        self.download_file(&download_info.url, &dest_path, on_progress).await?;

        if !crate::check_sha1_hash(&dest_path, download_info.sha1).map_err(JavaManagerError::Io)? {
            std::fs::remove_file(&dest_path)?;
            return Err(JavaManagerError::HashMismatch);
        }

        self.extract_java_archive(&dest_path, &runtime_dir).await?;

        std::fs::remove_file(&dest_path)?;

        let version_dir = runtime_dir.join(format!("{}-{}{}", version, arch, if is_jre { "-jre" } else { "" }));

        Ok(JavaLocation {
            path: version_dir,
            version: version.to_string(),
            arch: arch.to_string(),
            is_jre,
        })
    }

    async fn get_java_download_url(
        &self,
        version: &str,
        arch: &str,
        is_jre: bool,
    ) -> Result<JavaDownloadInfo, JavaManagerError> {
        let aj_url = format!("https://api.adoptium.net/v3/assets/latest/{}/hotspot", version);
        
        let response: serde_json::Value = self.http_client
            .get(&aj_url)
            .send()
            .await?
            .json()
            .await?;

        if let Some(assets) = response.get("assets").and_then(|v| v.as_array()) {
            for asset in assets {
                if let Some(image) = asset.get("image_type") {
                    let image_type = image.as_str().unwrap_or("");
                    if is_jre && image_type != "jre" || !is_jre && image_type != "jdk" {
                        continue;
                    }
                } else {
                    if is_jre { continue; }
                }

                if let Some(arch_obj) = asset.get("architecture") {
                    if let Some(arch_str) = arch_obj.as_str() {
                        let normalized_arch = match arch {
                            "x64" => "x64",
                            "x86" => "x86",
                            "arm64" => "aarch64",
                            _ => arch,
                        };
                        if arch_str != normalized_arch {
                            continue;
                        }
                    }
                }

                let binary = asset.get("binary");
                if let Some(binary) = binary {
                    let package = binary.get("package");
                    if let Some(package) = package {
                        let link = package.get("link").and_then(|l| l.as_str()).unwrap_or("");
                        let sha1 = package.get("sha1").and_then(|s| s.as_str()).unwrap_or("");

                        if !link.is_empty() {
                            let mut sha1_bytes = [0u8; 20];
                            hex::decode_to_slice(sha1, &mut sha1_bytes).map_err(|_| JavaManagerError::InvalidData(Arc::from("Invalid SHA1")))?;

                            return Ok(JavaDownloadInfo {
                                url: link.to_string(),
                                sha1: sha1_bytes,
                            });
                        }
                    }
                }
            }
        }

        Err(JavaManagerError::NotFound(format!("No matching Java build found for {}/{}/{}", version, arch, if is_jre { "jre" } else { "jdk" })))
    }

    async fn download_file(
        &self,
        url: &str,
        dest: &Path,
        on_progress: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
    ) -> Result<(), JavaManagerError> {
        let response = self.http_client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(JavaManagerError::DownloadFailed(Arc::from(format!("HTTP {}", response.status()))));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0usize;

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(dest)?;

        let bytes = response.bytes().await?;
        file.write_all(&bytes)?;
        downloaded += bytes.len();

        if let Some(progress_cb) = &on_progress {
            progress_cb(downloaded, total_size as usize);
        }

        Ok(())
    }

    async fn extract_java_archive(&self, archive: &Path, dest_dir: &Path) -> Result<(), JavaManagerError> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let file = std::fs::File::open(archive)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        archive.unpack(dest_dir)?;

        Ok(())
    }

    pub fn list_installed_java(&self) -> Vec<JavaLocation> {
        let mut result = Vec::new();
        let runtime_dir = self.get_runtime_dir();

        if !runtime_dir.exists() {
            return result;
        }

        let Ok(entries) = std::fs::read_dir(&runtime_dir) else {
            return result;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dir_name = entry.file_name().to_string_lossy().into_owned();
            
            let parts: Vec<&str> = dir_name.split('-').collect();
            if parts.len() >= 2 {
                let version = parts[0];
                let arch = if parts.len() >= 3 {
                    parts[1]
                } else {
                    parts[1]
                };

                let java_bin = if cfg!(windows) {
                    path.join("bin").join("java.exe")
                } else {
                    path.join("bin").join("java")
                };

                if java_bin.exists() {
                    result.push(JavaLocation {
                        path,
                        version: version.to_string(),
                        arch: arch.to_string(),
                        is_jre: dir_name.contains("-jre"),
                    });
                }
            }
        }

        result
    }
}

struct JavaDownloadInfo {
    url: String,
    sha1: [u8; 20],
}
