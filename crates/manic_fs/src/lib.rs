use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
use windows::{
    core::HSTRING,
    Storage::{FileProperties::BasicProperties, StorageFile},
};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use {nix::sys::stat, std::os::unix::prelude::AsRawFd};

#[cfg(target_os = "windows")]
pub async fn get_attrs(path: String) -> Result<BasicProperties, Box<dyn std::error::Error>> {
    let f: StorageFile = StorageFile::GetFileFromPathAsync(HSTRING::from(&path))?.await?;
    Ok(f.GetBasicPropertiesAsync()?.await?)
}

pub struct FileMetadata {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
