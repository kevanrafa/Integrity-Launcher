use std::sync::Arc;

use serde::Deserialize;

use crate::modrinth::{ModrinthHashes, ModrinthSideRequirement};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthModpackFileDownload {
    pub path: Arc<str>,
    pub hashes: ModrinthHashes,
    pub env: Option<ModrinthEnv>,
    pub downloads: Arc<[Arc<str>]>,
    pub file_size: usize,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ModrinthEnv {
    pub client: ModrinthSideRequirement,
}
