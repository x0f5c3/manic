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
//! - `rustls-tls`: Enables https through Rustls, enabled by default
//! - `openssl-tls`: Enables https through openssl
//!
//!
//! ## Crate usage
//! ```no_run
//! use manic::Downloader;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), manic::Error> {
//!     let number_of_concurrent_tasks: u8 = 5;
//!     let client = Downloader::new("https://crates.io", number_of_concurrent_tasks).await?;
//!     let result = client.download().await?;
//!     Ok(())
//! }
//! ```
#![deny(missing_debug_implementations)]
#![allow(dead_code)]
#![warn(missing_docs)]

pub use client::{Client, Response};
#[doc(inline)]
pub use downloader::Downloader;
#[doc(inline)]
pub use error::Error;
#[doc(inline)]
pub use error::Result;
#[doc(inline)]
pub use hash::Hash;

pub(crate) mod chunk;
/// This module is the main part of the crate
pub mod downloader;
/// Error definitions
pub mod error;
mod hash;
/// Client
pub mod client;
#[cfg(feature = "github")]
/// Interaction with github repos
pub mod github;

#[cfg(feature = "progress")]
pub use indicatif::ProgressStyle;
