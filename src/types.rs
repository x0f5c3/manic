use std::fmt;

#[cfg(feature = "openssl-tls")]
/// Alias for a Downloader with OpenSSL [`HttpsConnector`][hyper_tls::HttpsConnector]
pub type OpenSslDl = crate::downloader::Downloader<crate::OpenSSL>;
#[cfg(feature = "rustls-tls")]
/// Alias for a Downloader with Rustls [`HttpsConnector`][hyper_rustls::HttpsConnector]
pub type RustlsDl = crate::downloader::Downloader<crate::Rustls>;

/// Available checksum types
#[derive(Debug)]
pub enum Hash {
    /// Sha224 sum
    SHA224(String),
    /// Sha256 sum
    SHA256(String),
    /// Sha384 sum
    SHA384(String),
    /// Sha512 sum
    SHA512(String),
}
impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SHA224(val) | Self::SHA256(val) | Self::SHA384(val) | Self::SHA512(val) => {
                write!(f, "{}", val)
            }
        }
    }
}
