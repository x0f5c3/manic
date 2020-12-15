//! Fast and simple async downloads
//!
//! Provides easy to use functions to download a file using multiple async connections
//! while taking care to preserve integrity of the file and check it against a SHA256 sum


/// This module is the main part of the crate
pub mod download;
pub(crate) mod chunk;

