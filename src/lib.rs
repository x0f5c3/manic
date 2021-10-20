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
//! async fn main() -> Result<(), manic::ManicError> {
//!     let number_of_concurrent_tasks: u8 = 5;
//!     let client = Downloader::new("https://crates.io", number_of_concurrent_tasks).await?;
//!     let result = client.download().await?;
//!     Ok(())
//! }
//! ```

mod async_utils;
mod chunk;
mod client;
mod downloader;
mod error;
mod hash;
mod io;

pub(crate) use async_utils::{join_all, join_all_futures};
pub use client::Client;
pub use downloader::Downloader;
pub use downloader::MultiDownloader;
pub use error::{ManicError, Result};
pub use hash::Hash;
pub use hyper::header;
pub(crate) use hyper::http;
#[cfg(feature = "progress")]
pub use indicatif::ProgressStyle;
pub(crate) use io::MyCursor;
