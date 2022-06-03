#![allow(dead_code, unused_imports)]
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
//! - `async`: Enables the async downloader, on by default
//! - `threaded`: Enables the native thread based downloader
//! - `rustls`: Use rustls for HTTPS, on by default
//! - `openssl`: Use openssl for HTTPS
//!
//!
//!
//! ## Crate usage
//!
//! ### Async example
//!
//! ```no_run
//! use manic_http::Downloader;
//! #[tokio::main]
//! async fn main() -> Result<(), manic::ManicError> {
//!     let number_of_concurrent_tasks: u8 = 5;
//!     let client = Downloader::new("https://crates.io", number_of_concurrent_tasks).await?;
//!     let result = client.download().await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Native threading example
//!
//! ```no_run
//! use manic_http::threaded::Downloader;
//! # fn main() -> Result<(), manic::ManicError> {
//! let client = Downloader::new("https://crates.io", 5)?;
//! client.download()?;
//! Ok(())
//! # }
//! ```
#[macro_use]
extern crate derive_builder;

#[cfg(feature = "progress")]
pub use indicatif::ProgressStyle;
pub use reqwest::{header, Url};

#[cfg(feature = "async")]
#[doc(inline)]
pub use async_client::{Client, Downloader, MultiDownloader};
pub use error::{ManicError, Result};
#[cfg(all(not(feature = "async"), feature = "threaded"))]
#[doc(inline)]
pub use threaded::{Client, Downloader, MultiDownloader};

#[cfg(feature = "async")]
pub mod async_client;
mod error;

mod hash;
#[cfg(feature = "threaded")]
pub mod threaded;

pub use hash::Hash;
