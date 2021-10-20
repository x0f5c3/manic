use futures::{StreamExt, TryStreamExt};
use crate::header::CONTENT_LENGTH;
use crate::{ManicError, Result};
use hyper::Body;
use hyper::body::Buf;

use crate::header::LOCATION;
#[cfg(feature = "json")]
use serde::de::DeserializeOwned;

/// Wrapper for [`Response`][hyper::Response]
#[derive(Debug)]
pub struct Response(pub(crate) hyper::Response<Body>);

impl From<hyper::Response<Body>> for Response {
    fn from(resp: hyper::Response<Body>) -> Self {
        Self(resp)
    }
}

impl Response {
    /// Deserialize from json
    #[cfg(feature = "json")]
    pub async fn json<T: DeserializeOwned>(self) -> Result<T> {
        let full = hyper::body::to_bytes(self.0).await?;
        serde_json::from_slice(&full).map_err(|e| e.into())
    }
    /// Get the content-length from Response
    pub fn content_length(&self) -> Result<u64> {
        let heads = self.0.headers();
        let len = heads
            .get(CONTENT_LENGTH)
            .ok_or(ManicError::NoLen)?
            .to_str()?;
        len.parse::<u64>().map_err(Into::into)
    }
    pub async fn chunk(&self) -> Result<&[u8]> {
        self.0.into_body().into_stream().
    }
    /// Extract text from Response
    pub async fn text(self) -> Result<String> {
        let full = hyper::body::to_bytes(self.0).await?;
        String::from_utf8(full.to_vec()).map_err(|e| ManicError::UTF8(e))
    }
    /// Check if the response redirects
    pub fn check_redirect(&self, origin: &hyper::Uri) -> Result<Option<hyper::Uri>> {
        let status = self.0.status().as_u16();
        if status == 301 || status == 308 || status == 302 || status == 303 || status == 307 {
            let loc = self
                .0
                .headers()
                .get(LOCATION)
                .ok_or(ManicError::NoLoc)?
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
