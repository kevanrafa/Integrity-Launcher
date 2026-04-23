#![allow(dead_code)]

use std::path::Path;

pub(crate) fn ensure_directory(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}
