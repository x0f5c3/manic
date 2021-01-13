use std::num::ParseIntError;
use thiserror::Error;
use tokio::io;
use std::string::FromUtf8Error;

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
    #[error("hyper error: {0}")]
    NetError(#[from] hyper::Error),
    /// Returned when the header can't be parsed to a String
    #[error(transparent)]
    ToStr(#[from] hyper::header::ToStrError),
    /// Returned when there's no filename in the url
    #[error("No filename in url")]
    NoFilename(String),
    /// Returned when the url couldn't be parsed
    #[error("Failed to parse URL")]
    UrlParseError(#[from] http::uri::InvalidUri),
    /// Returned when the SHA256 sum didn't match
    #[error("Checksum doesn't match")]
    SHA256MisMatch(String),
    /// Returned when the selected chunk size == 0
    #[error("Invalid chunk size")]
    BadChunkSize,
    /// Error thrown when there's an error building a hyper request
    #[error("Request builder error: {0}")]
    REQError(#[from] http::Error),
    /// Failed creating an Uri from parts
    #[error("From parts error: {0}")]
    PartsError(#[from] http::uri::InvalidUriParts),
    #[cfg(feature = "github")]
    #[error("Serde error: {0}")]
    SerError(#[from] serde_json::Error),
    #[error("UTF8 Error: {0}")]
    UTF8(#[from] FromUtf8Error),
}

/// Alias for Result<T, manic::Error>
pub type Result<T> = std::result::Result<T, Error>;
