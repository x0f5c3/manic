mod error;
mod chunk;
mod downloader;
mod hash;
mod multi;

pub use downloader::Downloader;
pub use downloader::DownloaderBuilder;
pub use error::{ManicError, Result};
pub use hash::Hash;
pub use multi::Downloaded;
pub use multi::Map;
pub use multi::MultiDownloader;
pub use multi::MultiDownloaderBuilder;
pub use reqwest::Client;