use crate::header::CONTENT_LENGTH;
use crate::header::LOCATION;
use crate::{ManicError, Result};
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use hyper::client::HttpConnector;
use hyper::Body;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
#[cfg(feature = "json")]
use serde::de::DeserializeOwned;

pub(crate) struct Client {
    hyper: hyper::Client<HttpsConnector<HttpConnector>>,
    redirects: bool,
}

impl Client {
    pub fn new(redirect: bool) -> Self {
        let conn = HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .build();
        let hyp = hyper::Client::builder().build(conn);
        Self {
            hyper: hyp,
            redirects: redirect,
        }
    }
}

/// Wrapper for [`Response`][hyper::Response]
#[derive(Debug)]
pub struct Response {
    pub(crate) resp: hyper::Response<Body>,
    pub(crate) url: hyper::Uri,
}

impl Response {
    pub fn from_hyper(resp: hyper::Response<Body>, url: hyper::Uri) -> Self {
        Self { resp, url }
    }
    /// Deserialize from json
    #[cfg(feature = "json")]
    pub async fn json<T: DeserializeOwned>(self) -> Result<T> {
        let full = hyper::body::to_bytes(self.resp).await?;
        serde_json::from_slice(&full).map_err(|e| e.into())
    }
    /// Get the content-length from Response
    pub fn content_length(&self) -> Result<u64> {
        let heads = self.resp.headers();
        let len = heads
            .get(CONTENT_LENGTH)
            .ok_or(ManicError::NoLen)?
            .to_str()?;
        len.parse::<u64>().map_err(Into::into)
    }
    pub async fn chunk(&mut self) -> Result<Option<Bytes>> {
        if let Some(item) = self.resp.body_mut().next().await {
            Ok(Some(item?))
        } else {
            Ok(None)
        }
    }
    /// Extract text from Response
    pub async fn text(self) -> Result<String> {
        let full = hyper::body::to_bytes(self.resp).await?;
        String::from_utf8(full.to_vec()).map_err(ManicError::UTF8)
    }
    /// Check if the response redirects
    pub fn check_redirect(&self, origin: &hyper::Uri) -> Result<Option<hyper::Uri>> {
        let status = self.resp.status().as_u16();
        if status == 301 || status == 308 || status == 302 || status == 303 || status == 307 {
            let loc = self
                .resp
                .headers()
                .get(LOCATION)
                .ok_or(ManicError::NoLen)?
                .to_str()?;
            let uri = loc.parse::<hyper::Uri>()?;
            return if uri.host().is_some() {
                Ok(Some(uri))
            } else {
                let cloned = origin.clone();
                let mut part = cloned.into_parts();
                let new_path = uri
                    .path_and_query()
                    .ok_or(ManicError::BadChunkSize)?
                    .to_owned();
                part.path_and_query = Some(new_path);
                let new_url = hyper::Uri::from_parts(part)?;
                Ok(Some(new_url))
            };
        } else {
            Ok(None)
        }
    }
}
