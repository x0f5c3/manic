pub use reqwest::Client;

pub use downloader::Downloader;
pub use downloader::DownloaderBuilder;
pub use multi::Downloaded;
pub use multi::Map;
pub use multi::MultiDownloader;
pub use multi::MultiDownloaderBuilder;

mod chunk;
mod downloader;
#[cfg(feature = "hyper_client")]
pub mod hyper_client;
mod multi;
