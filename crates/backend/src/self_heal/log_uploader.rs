use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LogUploaderError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Failed to upload: {0}")]
    UploadFailed(Arc<str>),
    #[error("Invalid response")]
    InvalidResponse,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogMetadata {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UploadResponse {
    pub id: String,
    pub url: String,
    pub raw: String,
}

pub struct LogUploader {
    http_client: reqwest::Client,
}

impl LogUploader {
    pub fn new(http_client: reqwest::Client) -> Self {
        Self { http_client }
    }

    pub async fn upload_logs(
        &self,
        logs: Vec<LogMetadata>,
    ) -> Result<Option<UploadResponse>, LogUploaderError> {
        let mut form = HashMap::new();

        for (i, log) in logs.iter().enumerate() {
            form.insert(format!("log[{}]", i), log.content.clone());
            form.insert(format!("title[{}]", i), log.name.clone());
        }

        let response = self.http_client
            .post("https://api.mclo.gs/1/log")
            .header("User-Agent", "IntegrityLauncher")
            .form(&form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LogUploaderError::UploadFailed(Arc::from(format!("HTTP {}", response.status()))));
        }

        let result: serde_json::Value = response.json().await?;

        if let Some(id) = result.get("id").and_then(|v| v.as_str()) {
            let url = format!("https://mclo.gs/{}", id);
            let raw = format!("https://api.mclo.gs/1/log/{}", id);

            Ok(Some(UploadResponse {
                id: id.to_string(),
                url,
                raw,
            }))
        } else {
            Err(LogUploaderError::InvalidResponse)
        }
    }

    pub async fn upload_single_log(
        &self,
        name: &str,
        content: &str,
    ) -> Result<Option<UploadResponse>, LogUploaderError> {
        self.upload_logs(vec![LogMetadata {
            name: name.to_string(),
            content: content.to_string(),
        }]).await
    }

    pub async fn upload_from_files(
        &self,
        files: &[(&Path, &str)],
    ) -> Result<Option<UploadResponse>, LogUploaderError> {
        let mut logs = Vec::new();

        for (path, name) in files {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                logs.push(LogMetadata {
                    name: name.to_string(),
                    content,
                });
            }
        }

        self.upload_logs(logs).await
    }
}
