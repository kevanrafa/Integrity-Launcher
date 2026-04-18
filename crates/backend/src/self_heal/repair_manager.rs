use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use crate::directories::LauncherDirectories;
use crate::self_heal::log_uploader::LogUploaderError;

#[derive(Error, Debug)]
pub enum RepairError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Library validation failed: {0}")]
    LibraryValidation(String),
    #[error("Native extraction failed: {0}")]
    NativeExtraction(String),
    #[error("Java setup failed: {0}")]
    JavaSetup(String),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Missing required component: {0}")]
    MissingComponent(String),
    #[error("Log upload error: {0}")]
    LogUpload(#[from] LogUploaderError),
}

pub struct RepairManager {
    java_manager: super::java_manager::JavaManager,
    library_manager: super::library_manager::LibraryManager,
    natives_manager: super::natives_manager::NativesManager,
    log_uploader: super::log_uploader::LogUploader,
}

impl RepairManager {
    pub fn new(
        http_client: reqwest::Client,
        directories: Arc<LauncherDirectories>,
    ) -> Self {
        let java_manager = super::java_manager::JavaManager::new(http_client.clone(), Arc::clone(&directories));
        let library_manager = super::library_manager::LibraryManager::new(http_client.clone(), Arc::clone(&directories));
        let natives_manager = super::natives_manager::NativesManager::new(&directories);
        let log_uploader = super::log_uploader::LogUploader::new(http_client);

        Self {
            java_manager,
            library_manager,
            natives_manager,
            log_uploader,
        }
    }

    pub async fn repair_instance(
        &self,
        instance_id: &str,
        java_version: Option<&str>,
        on_progress: Option<Box<dyn Fn(&str, usize, usize) + Send + Sync>>,
    ) -> Result<RepairResult, RepairError> {
        let mut result = RepairResult::default();

        if let Some(java_ver) = java_version {
            if let Some(cb) = &on_progress {
                cb("Installing Java runtime", 0, 100);
            }

            match self.java_manager.download_java(java_ver, "x64", false, None).await {
                Ok(java) => {
                    result.java_installed = Some(java.version);
                }
                Err(e) => {
                    return Err(RepairError::JavaSetup(e.to_string()));
                }
            }
        }

        if let Some(cb) = &on_progress {
            cb("Validating libraries", 0, 100);
        }
        
        self.repair_libraries().await?;

        if let Some(cb) = &on_progress {
            cb("Repair complete", 100, 100);
        }

        Ok(result)
    }

    async fn repair_libraries(&self) -> Result<(), RepairError> {
        self.library_manager.cleanup_orphaned_libraries(&[]);
        Ok(())
    }

    pub async fn upload_logs(
        &self,
        log_files: &[(&Path, &str)],
    ) -> Result<Option<String>, RepairError> {
        let response = self.log_uploader.upload_from_files(log_files).await?;
        Ok(response.map(|r| r.url))
    }

    pub fn get_natives_manager(&self) -> &super::natives_manager::NativesManager {
        &self.natives_manager
    }

    pub fn get_java_manager(&self) -> &super::java_manager::JavaManager {
        &self.java_manager
    }

    pub fn get_library_manager(&self) -> &super::library_manager::LibraryManager {
        &self.library_manager
    }
}

#[derive(Debug, Default)]
pub struct RepairResult {
    pub java_installed: Option<String>,
    pub libraries_repaired: usize,
    pub natives_repaired: usize,
    pub config_fixed: bool,
}
