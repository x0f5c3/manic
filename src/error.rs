use crate::http;
use crate::http::uri::{InvalidUri, InvalidUriParts};
use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use std::num::ParseIntError;
use std::string::FromUtf8Error;
use tokio::io;
use url::ParseError;

/// Error definition for possible errors in this crate
#[derive(Debug, Clone)]
pub enum ManicError {
    /// Returned when the content length couldn't be parsed
    LenParse,
    /// Returned when the content-length = 0
    NoLen,
    /// Represents problems with Tokio based IO
    TokioIOError(String),
    /// Represents problems with network connectivity
    NetError(String),
    /// Returned when the header can't be parsed to a String
    ToStr(String),
    /// Returned when there's no filename in the url
    NoFilename(String),
    /// Returned when the url couldn't be parsed
    UrlParseError(String),
    /// Returned when the SHA256 sum didn't match
    SHA256MisMatch(String),
    /// Returned when the selected chunk size == 0
    BadChunkSize,
    NotFound,
    NoLoc,
    #[cfg(feature = "json")]
    SerError(String),
    MultipleErrors(String),
    NoResults,
    HTTPError(String),
    InvalidUri(String),
    UTF8(String),
    InvalidUriParts(String),
}

impl Error for ManicError {}

#[cfg(feature = "json")]
impl From<serde_json::Error> for ManicError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerError(e.to_string())
    }
}

impl From<FromUtf8Error> for ManicError {
    fn from(e: FromUtf8Error) -> Self {
        Self::UTF8(e.to_string())
    }
}

impl From<InvalidUriParts> for ManicError {
    fn from(e: InvalidUriParts) -> Self {
        Self::InvalidUriParts(e.to_string())
    }
}

impl From<hyper::http::Error> for ManicError {
    fn from(e: hyper::http::Error) -> Self {
        Self::HTTPError(e.to_string())
    }
}

impl fmt::Display for ManicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::LenParse => write!(f, "Failed to parse content-length"),
            Self::NoLen => write!(f, "Failed to retrieve content-length"),
            Self::TokioIOError(s) => write!(f, "Tokio IO error: {}", s),
            Self::NetError(s) => write!(f, "Hyper error: {}", s),
            Self::ToStr(s) => write!(f, "Couldn't parse the header into string: {}", s),
            Self::NoFilename(s) => write!(f, "No filename in url: {}", s),
            Self::UrlParseError(s) => write!(f, "Failed to parse URL: {}", s),
            Self::SHA256MisMatch(s) => write!(f, "Checksum doesn't match: {}", s),
            Self::BadChunkSize => write!(f, "Invalid chunk size"),
            Self::NotFound => write!(f, "Downloader not found"),
            Self::MultipleErrors(s) => write!(f, "{}", s),
            Self::NoResults => write!(f, "No errors and no results from join_all"),
            Self::HTTPError(s) => write!(f, "HTTP error: {}", s),
            Self::InvalidUri(s) => write!(f, "Invalid URI: {}", s),
            Self::InvalidUriParts(s) => write!(f, "Invalid URI parts: {}", s),
            Self::NoLoc => write!(f, "No location header found"),
        }
    }
}

impl From<hyper::http::uri::InvalidUri> for ManicError {
    fn from(e: InvalidUri) -> Self {
        Self::InvalidUri(e.to_string())
    }
}

impl From<ParseIntError> for ManicError {
    fn from(_: ParseIntError) -> Self {
        Self::LenParse
    }
}

impl From<io::Error> for ManicError {
    fn from(e: std::io::Error) -> Self {
        Self::TokioIOError(e.to_string())
    }
}

impl From<hyper::Error> for ManicError {
    fn from(e: hyper::Error) -> Self {
        Self::NetError(e.to_string())
    }
}

impl From<url::ParseError> for ManicError {
    fn from(e: ParseError) -> Self {
        Self::UrlParseError(e.to_string())
    }
}

impl From<hyper::header::ToStrError> for ManicError {
    fn from(e: hyper::header::ToStrError) -> Self {
        Self::ToStr(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ManicError>;

impl<I: Into<ManicError>> From<Vec<I>> for ManicError {
    fn from(errs: Vec<I>) -> Self {
        let mut msg = String::new();
        for i in errs {
            msg += &format!("- [{}]", i.into());
        }
        Self::MultipleErrors(msg)
    }
}
