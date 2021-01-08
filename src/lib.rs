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
//! // On feature `rustls-tls`
//! # #[cfg(feature = "rustls-tls")]
//! use manic::Rustls;
//! // On feature `openssl-tls`
//! # #[cfg(feature = "openssl-tls")]
//! use manic::OpenSSL;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), manic::Error> {
//!     let number_of_concurrent_tasks: u8 = 5;
//!     // With `Rustls` if both features are enabled
//!     # #[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
//!     let client = Downloader::<Rustls>::new("https://crates.io", number_of_concurrent_tasks).await?;
//!
//!     // With both features enabled also Openssl can be chosen like this:
//!     # #[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
//!     let client = Downloader::<OpenSSL>::new("https://crates.io", number_of_concurrent_tasks).await?;
//!
//!     // If only one of the two TLS features is enabled,
//!     // there's no need to specify the connector type, just need to use a convenient type alias
//!     # #[cfg(any(all(feature = "rustls-tls", not(feature = "openssl-tls"), all(feature = "openssl-tls", not(feature = "rustls-tls")))))]
//!     let client = Downloader::new("https://crates.io", number_of_concurrent_tasks).await?;
//!     let result = client.download().await?;
//!     Ok(())
//! }
//! ```
#![deny(missing_debug_implementations)]
#![allow(dead_code)]
#![deny(missing_docs)]


pub(crate) mod chunk;
/// This module is the main part of the crate
pub mod downloader;
mod utils;
/// Error definitions
pub mod error;
mod connector;
mod types;
#[doc(inline)]
pub use error::Error;
#[doc(inline)]
pub use error::Result;
#[doc(inline)]
pub use types::Hash;
#[doc(inline)]
pub use connector::Connector;

#[cfg(any(all(feature = "openssl-tls", not(feature = "rustls-tls")),all(feature = "rustls-tls", not(feature = "openssl-tls"))))]
pub use types::Downloader;

#[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
pub use downloader::Downloader;

#[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
pub use connector::{Rustls, OpenSSL};




