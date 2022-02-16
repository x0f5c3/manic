mod chunk;
pub mod downloader;
pub mod multi;

#[doc(inline)]
pub use downloader::Downloader;
#[cfg(feature = "progress")]
pub use indicatif::ProgressStyle;
pub use multi::MultiDownloader;
pub use reqwest::blocking::Client;
