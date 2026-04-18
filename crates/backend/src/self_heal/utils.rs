use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "macos")]
use std::process::Command;

pub use crate::directories::LauncherDirectories;

#[derive(Error, Debug)]
pub enum SelfHealError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Failed to download: {0}")]
    DownloadFailed(Arc<str>),
    #[error("Hash mismatch for: {0}")]
    HashMismatch(String),
    #[error("Invalid data: {0}")]
    InvalidData(Arc<str>),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Unsupported platform")]
    UnsupportedPlatform,
}

pub fn ensure_dir_exists(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn compute_sha1(path: &Path) -> Result<[u8; 20], SelfHealError> {
    use sha1::{Digest, Sha1};
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(*hasher.finalize().as_ref())
}

pub fn verify_hash(path: &Path, expected_hash: &[u8; 20]) -> Result<bool, SelfHealError> {
    let actual = compute_sha1(path)?;
    Ok(actual == *expected_hash)
}

pub fn temp_file_path(path: &Path) -> PathBuf {
    let mut temp = path.to_path_buf();
    use rand::RngCore;
    temp.add_extension(format!("{}", rand::thread_rng().next_u32()));
    temp.set_extension(format!("{}.new", temp.extension().unwrap_or_default().to_string_lossy()));
    temp
}

pub fn get_open_command() -> (&'static str, &'static [&'static str]) {
    #[cfg(target_os = "windows")]
    {
        ("cmd", &["/C", "start", ""])
    }
    #[cfg(target_os = "macos")]
    {
        ("open", &[])
    }
    #[cfg(target_os = "linux")]
    {
        ("xdg-open", &[])
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        compile_error!("Unsupported platform")
    }
}

pub fn open_game_directory(path: &Path) -> Result<(), SelfHealError> {
    let (cmd, args) = get_open_command();
    let path_str = path.to_string_lossy();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new(cmd)
            .args(args)
            .arg(&*path_str)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| SelfHealError::Io(e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new(cmd)
            .arg(&*path_str)
            .spawn()
            .map_err(|e| SelfHealError::Io(e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new(cmd)
            .arg(&*path_str)
            .spawn()
            .map_err(|e| SelfHealError::Io(e))?;
    }

    Ok(())
}

pub fn create_required_folders(directories: &LauncherDirectories) -> Result<(), SelfHealError> {
    let folders = [
        directories.temp_dir.as_ref(),
        directories.temp_natives_base_dir.as_ref(),
        directories.runtime_base_dir.as_ref(),
        directories.log_configs_dir.as_ref(),
    ];

    for folder in folders {
        std::fs::create_dir_all(folder).map_err(SelfHealError::Io)?;
    }

    Ok(())
}
