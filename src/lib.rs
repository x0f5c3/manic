//! Fast and simple async downloads
//!
//! Provides easy to use functions to download a file using multiple async connections
//! while taking care to preserve integrity of the file and check it against a SHA256 sum
//!
//! This crate is a work in progress
//!
//!
//! The crate exposes debug logs through the [`tracing`][tracing] crate
//!
//! ## Feature flags
//!
//! - `progress`: Enables progress reporting using `indicatif`
//! - `json`: Enables use of JSON features on the reqwest [`Client`][reqwest::Client]
//!
//!
//! ## Crate usage
//!
//! # Example
//!
//! ```no_run
//! use manic::Downloader;
//! #[tokio::main]
//! async fn main() -> Result<(), manic::Error> {
//!     let number_of_concurrent_tasks: u8 = 5;
//!     let client = Downloader::new("https://crates.io", number_of_concurrent_tasks).await?;
//!     let result = client.download().await?;
//!     Ok(())
//! }
//! ```

mod chunk;
mod downloader;
mod error;
mod hash;


pub use downloader::Downloader;
pub use error::{Error, Result};
pub use hash::Hash;
#[cfg(feature = "progress")]
pub use indicatif::ProgressStyle;
pub use reqwest::Client;
pub use reqwest::Url;
