mod chunk;
pub mod downloader;
mod error;
mod hash;
mod multi;

pub use downloader::Downloader;
pub use error::{ManicError, Result};
pub use hash::Hash;
#[cfg(feature = "progress")]
pub use indicatif::ProgressStyle;
pub use reqwest::blocking::Client;
