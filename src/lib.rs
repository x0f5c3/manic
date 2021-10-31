//! Fast and simple async_client downloads
//!
//! Provides easy to use functions to download a file using multiple async_client connections
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
//! async_client fn main() -> Result<(), manic::ManicError> {
//!     let number_of_concurrent_tasks: u8 = 5;
//!     let client = Downloader::new("https://crates.io", number_of_concurrent_tasks).await?;
//!     let result = client.download().await?;
//!     Ok(())
//! }
//! ```
#[macro_use]
extern crate derive_builder;
#[cfg(feature = "threaded")]
pub mod threaded;
#[cfg(feature = "async")]
pub mod async_client;

#[cfg(feature = "progress")]
pub use indicatif::ProgressStyle;
pub use reqwest::{header, Url};
