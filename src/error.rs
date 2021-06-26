use std::num::ParseIntError;
use thiserror::Error;
use tokio::io;

/// Error definition for possible errors in this crate
#[derive(Debug, Error)]
pub enum Error {
    /// Returned when the content length couldn't be parsed
    #[error("Failed to parse content-length")]
    LenParse(#[from] ParseIntError),
    /// Returned when the content-length = 0
    #[error("Failed to retrieve content-length")]
    NoLen,
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
    /// Returned when the string couldn't be parsed to a [`HeaderValue`][reqwest::header::HeaderValue]
    #[error("Invalid header value: {0}")]
    HeaderVal(#[from] reqwest::header::InvalidHeaderValue),
}
pub type Result<T> = std::result::Result<T, Error>;
