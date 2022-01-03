use std::fmt::Display;
use std::num::ParseIntError;
use thiserror::Error;

/// Error definition for possible errors in this crate
#[derive(Debug, Error)]
pub enum ManicError {
    /// Returned when the content length couldn't be parsed
    #[error("Failed to parse content-length")]
    LenParse(#[from] ParseIntError),
    /// Returned when the content-length = 0
    #[error("Content length is 0")]
    NoLen,
    /// Represents problems with IO
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Network error: {0}")]
    NetError(#[from] reqwest::Error),
    /// Returned when the header can't be parsed to a String
    #[error("Header to string error: {0}")]
    ToStr(#[from] reqwest::header::ToStrError),
    /// Returned when there's no filename in the url
    #[error("No filename in url {0}")]
    NoFilename(String),
    /// Returned when the url couldn't be parsed
    #[error("URL parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),
    /// Returned when the SHA256 sum didn't match
    #[error("SHA sum mismatch: {0}")]
    SHA256MisMatch(String),
    /// Returned when the selected chunk size == 0
    #[error("Chunk size cannot be 0")]
    BadChunkSize,
    #[error("Not found")]
    NotFound,
    #[error("No results found")]
    NoResults,
    #[cfg(feature = "threaded")]
    #[error("Canceled: {0}")]
    Canceled(#[from] futures_channel::oneshot::Canceled),
    #[cfg(feature = "async")]
    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("PoisonError: {0}")]
    PoisonError(String),
    #[error("{0}")]
    MultipleErrors(String),
    #[error("Hyper error: {0}")]
    HyperErr(#[from] hyper::Error),
}

pub type Result<T> = std::result::Result<T, ManicError>;

impl<I: Into<ManicError> + Display> From<Vec<I>> for ManicError {
    fn from(errs: Vec<I>) -> Self {
        let mut msg = String::new();
        for i in errs {
            msg += &format!("- [{}]\n", i);
        }
        Self::MultipleErrors(msg)
    }
}
