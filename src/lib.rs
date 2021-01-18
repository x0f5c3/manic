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
//!
//!
//! ## Crate usage
//!
//! # Example
//!
//! ```no_run
//!
//! use manic::downloader;
//! use reqwest::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), manic::Error> {
//!     let client = Client::new();
//!     let number_of_concurrent_tasks: u8 = 5;
//!     let result = downloader::download(&client, "https://crates.io", number_of_concurrent_tasks).await?;
//!     Ok(())
//! }
//! ```
//!
//!


/// This module is the main part of the crate
pub mod downloader;
/// Only available on feature `progress`
#[cfg(feature = "progress")]
pub mod progress;
pub mod chunk;
mod hash;
mod error;
pub use error::{Error, Result};
pub use hash::Hash;

#[cfg(feature = "progress")]
pub use indicatif::ProgressStyle;
