use crate::{Error, Result};
use async_trait::async_trait;
use hyper::client::connect::Connect;
use hyper::header::{CONTENT_LENGTH, LOCATION};
#[cfg(feature = "github")]
use serde::Deserialize;
use tracing::debug;

/// Trait implemented for HTTPS connectors
pub trait Connector {
    /// Construct the HttpsConnector
    fn new() -> Self;
}

#[cfg(feature = "rustls-tls")]
impl Connector for crate::Rustls {
    fn new() -> Self {
        hyper_rustls::HttpsConnector::with_native_roots()
    }
}

#[cfg(feature = "openssl-tls")]
impl Connector for crate::OpenSSL {
    fn new() -> Self {
        hyper_tls::HttpsConnector::new()
    }
}
#[doc(hidden)]
#[async_trait]
pub trait ClientExt {
    /// Check for redirects and return the new url or the old one
    async fn check_redirects(&self, url: hyper::Uri) -> Result<hyper::Uri>;
    async fn content_length(&self, url: &hyper::Uri) -> Result<u64>;
}

#[async_trait]
impl<C> ClientExt for hyper::Client<C>
where
    C: Connect + Send + Sync + Clone + 'static,
{
    /// Check for redirects and return the new url or the old one
    async fn check_redirects(&self, url: hyper::Uri) -> Result<hyper::Uri>
where {
        let req = hyper::Request::head(&url)
            .body(hyper::Body::empty())
            .map_err(|e| Error::REQError(e))?;
        let head_req = self.request(req.into()).await?;
        let status = head_req.status().as_u16();
        if status == 301 || status == 308 || status == 302 || status == 303 || status == 307 {
            let loc = head_req.headers()[LOCATION].to_str()?;
            let uri = loc.parse::<hyper::Uri>()?;
            return if uri.host().is_some() {
                Ok(uri)
            } else {
                let mut part = url.into_parts();
                let new_path = uri.path_and_query().ok_or(Error::BadChunkSize)?.to_owned();
                part.path_and_query = Some(new_path);
                let new_url = hyper::Uri::from_parts(part)?;
                Ok(new_url)
            };
        } else {
            Ok(url)
        }
    }

    /// Get the content-length header using a head request
    ///
    /// # Arguments
    ///
    /// * `url` - reference to [`hyper::Uri`][hyper::Uri]
    ///
    /// # Example
    ///
    ///```no_run
    /// # #[cfg(feature = "rustls-tls")]
    /// use manic::Rustls as Conn;
    /// # #[cfg(all(feature = "openssl-tls", not(feature = "rustls-tls")))]
    /// use manic::OpenSSL as Conn;
    /// use manic::Error;
    /// use hyper::Client;
    /// use manic::ClientExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    ///     let connector = Conn::new();
    ///     let client = Client::builder().build::<_, hyper::Body>(connector);
    ///     let url = "https://docs.rs".parse::<hyper::Uri>()?;
    ///     let length = client.get_length(&url).await?;
    ///     println!("{}", length);
    /// #   Ok(())
    /// # }
    /// ```
    async fn content_length(&self, url: &hyper::Uri) -> Result<u64> {
        let req = hyper::Request::head(url)
            .body(hyper::Body::empty())
            .map_err(|e| Error::REQError(e))?;
        let head_req = self.request(req.into()).await?;
        let headers = head_req.headers();
        debug!("Received head response: {:?}", headers);
        headers[CONTENT_LENGTH]
            .to_str()?
            .parse::<u64>()
            .map_err(Into::into)
    }
}
