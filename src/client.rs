use hyper::header::{CONTENT_LENGTH, LOCATION};
use hyper::{Body, Request};
#[cfg(feature = "json")]
use serde::de::DeserializeOwned;

use crate::Error;
use crate::Result;
use hyper::client::HttpConnector;
use tracing::debug;
use hyper_rustls::HttpsConnector;

#[macro_use]
macro_rules! head {
    ($url:expr) => {
        Request::head($url)
            .body(Body::empty())
            .map_err(|e| Error::REQError(e))
    };
    (&url:ident) => {
        Request::head($url)
            .body(Body::empty())
            .map_err(|e| Error::REQError(e))
    };
}

/// Wrapper for [`hyper::Client`][hyper::Client]
#[derive(Debug, Clone)]
pub struct Client {
    client: hyper::Client<HttpsConnector<HttpConnector>>,
    redirects: bool,
}

impl Client {
    /// Construct a new client, follows redirects by default
    pub fn new() -> Self {
        let client = hyper::Client::builder().build::<_, Body>(hyper_rustls::HttpsConnector::with_native_roots());
        Self {
            client,
            redirects: true,
        }
    }
    /// Follow redirects
    pub fn follow_redirects(&mut self, yes: bool) {
        self.redirects = yes;
    }
    /// Get the content-length from the url
    pub async fn content_length(&self, url: &str) -> Result<u64> {
        let head = self.head(url).await?;
        let res = head.content_length();
        debug!("Result {:?}", res);
        res
    }
    /// Make the custom request
    pub async fn request(&self, req: hyper::Request<Body>) -> Result<Response> {
        self.client
            .request(req)
            .await
            .map(|e| e.into())
            .map_err(|e| Error::NetError(e))
    }
    /// Perform a HEAD request
    pub async fn head(&self, url: &str) -> Result<Response> {
        let parsed = url.parse::<hyper::Uri>()?;
        if self.redirects {
            let req = head!(&parsed)?;
            let resp: Response = self.client
                .request(req.into())
                .await
                .map(|x| x.into())?;
            let check = resp.check_redirect(&parsed)?;
            if let Some(new) = check {
                debug!("Redirect: {}", new);
                let req = head!(&new)?;
                self.client.request(req.into()).await.map(|x| x.into()).map_err(|e| Error::NetError(e))
            } else {
                Ok(resp)
            }
        } else {
            let req = head!(&parsed)?;
            self.client
                .request(req.into())
                .await
                .map(|x| x.into())
                .map_err(|e| Error::NetError(e))
        }
    }
    /// Perform a GET request
    pub async fn get(&self, url: &str) -> Result<Response> {
        let parsed = url.parse::<hyper::Uri>()?;
        if self.redirects {
            let resp: Response = self.client
                .get(parsed.clone())
                .await
                .map(|x| x.into())?;
            let check = resp.check_redirect(&parsed)?;
            if let Some(new) = check {
                self.client.get(new).await.map(|x| x.into()).map_err(|e| Error::NetError(e))
            } else {
                Ok(resp)
            }
        } else {
            self.client
                .get(parsed)
                .await
                .map(|x| x.into())
                .map_err(|e| Error::NetError(e))
        }
    }
}

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
        serde_json::from_slice(&full).map_err(|e| Error::SerError(e))
    }
    /// Get the content-length from Response
    pub fn content_length(&self) -> Result<u64> {
        let heads = self.0.headers();
        let len = heads.get(CONTENT_LENGTH).ok_or(Error::NoLen)?.to_str()?;
        len.parse::<u64>().map_err(Into::into)
    }
    /// Extract text from Response
    pub async fn text(self) -> Result<String> {
        let full = hyper::body::to_bytes(self.0).await?;
        String::from_utf8(full.to_vec()).map_err(|e| Error::UTF8(e))
    }
    /// Check if the response redirects
    pub fn check_redirect(&self, origin: &hyper::Uri) -> Result<Option<hyper::Uri>> {
        let status = self.0.status().as_u16();
        if status == 301 || status == 308 || status == 302 || status == 303 || status == 307 {
            let loc = self.0.headers()[LOCATION].to_str()?;
            let uri = loc.parse::<hyper::Uri>()?;
            return if uri.host().is_some() {
                Ok(Some(uri))
            } else {
                let cloned = origin.clone();
                let mut part = cloned.into_parts();
                let new_path = uri.path_and_query().ok_or(Error::BadChunkSize)?.to_owned();
                part.path_and_query = Some(new_path);
                let new_url = hyper::Uri::from_parts(part)?;
                Ok(Some(new_url))
            };
        } else {
            Ok(None)
        }
    }

}
