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

use std::num::ParseIntError;
use thiserror::Error;
use tokio::io;

/// This module is the main part of the crate
pub mod downloader;
/// Only available on feature `progress`
#[cfg(feature = "progress")]
pub mod progress;
pub mod chunk;

#[cfg(feature = "progress")]
pub use indicatif::ProgressStyle;
/// Error definition for possible errors in this crate
#[derive(Debug, Error)]
pub enum Error {
    /// Returned when the content length couldn't be parsed
    #[error("Failed to parse content-length")]
    LenParse(#[from] ParseIntError),
    /// Represents problems with Tokio based IO
    #[error("Tokio IO error: {0}")]
    TokioIOError(#[from] io::Error),
    /// Represents problems with network connectivity
    #[error("Reqwest error: {0}")]
    NetError(#[from] reqwest::Error),
    /// Returned when the header can't be parsed to a String
    #[error(transparent)]
    ToStr(#[from] reqwest::header::ToStrError),
    /// Returned when there's no filename in the url
    #[error("No filename in url")]
    NoFilename(String),
    /// Returned when the url couldn't be parsed
    #[error("Failed to parse URL")]
    UrlParseError(#[from] url::ParseError),
    /// Returned when the SHA256 sum didn't match
    #[error("Checksum doesn't match")]
    SHA256MisMatch(String),
    /// Returned when the selected chunk size == 0
    #[error("Invalid chunk size")]
    BadChunkSize,
}
