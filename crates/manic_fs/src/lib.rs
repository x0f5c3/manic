use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use time::Time;

#[cfg(target_os = "windows")]
use windows::{
    core::HSTRING,
    Storage::{FileProperties::BasicProperties, StorageFile},
};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use {nix::sys::stat, std::os::unix::prelude::AsRawFd};

#[cfg(target_os = "windows")]
pub async fn get_attrs(path: String) -> Result<BasicProperties> {
    let f: StorageFile = StorageFile::GetFileFromPathAsync(HSTRING::from(&path))?.await?;
    Ok(f.GetBasicPropertiesAsync()?.await?)
}

#[cfg(not(target_os = "windows"))]
pub async fn get_attrs(path: String) -> Result<FileMetadata> {}

#[derive(Deserialize, Serialize)]
pub struct FileMetadata {
    name: String,
    relative_path: PathBuf,
    size: u64,
    modified: Time,
    created: Time,
}
