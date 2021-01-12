use crate::Error;
use crate::Result;
use hyper::client::connect::Connect;
use hyper::client::HttpConnector;
use hyper::header::LOCATION;
use hyper::{Body, Request, Response};
use serde::Deserialize;
use async_trait::async_trait;
use futures::StreamExt;
use hyper::body::Bytes;
use serde::de::DeserializeOwned;
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
#[derive(Debug, Clone)]
pub struct Builder<C>
where
    C: Connect + Send + Sync + Clone,
{
    redirects: bool,
    connector: C,
}

#[cfg(feature = "rustls-tls")]
impl Builder<hyper_rustls::HttpsConnector<HttpConnector>> {
    pub fn rustls() -> Self {
        Self::new(hyper_rustls::HttpsConnector::with_native_roots())
    }
}

#[cfg(feature = "openssl-tls")]
impl Builder<hyper_tls::HttpsConnector<HttpConnector>> {
    pub fn openssl() -> Self {
        Self::new(hyper_tls::HttpsConnector::new())
    }
}

impl<C> Builder<C>
where
    C: Connect + Send + Sync + Clone,
{
    pub fn new(c: C) -> Self {
        Self {
            redirects: false,
            connector: c,
        }
    }
    pub fn follow_redirects(mut self) -> Self {
        self.redirects = true;
        self
    }
    pub fn build(self) -> Client<C> {
        let client = hyper::Client::builder().build::<_, Body>(self.connector);
        Client {
            client,
            redirects: self.redirects,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Client<C>
where
    C: Connect + Send + Sync + Clone,
{
    client: hyper::Client<C>,
    redirects: bool,
}

#[cfg(feature = "rustls-tls")]
impl Client<hyper_rustls::HttpsConnector<HttpConnector>> {
    pub fn new_rustls() -> Self {
        let conn = hyper_rustls::HttpsConnector::with_native_roots();
        let client = hyper::Client::builder().build(conn);
        Self {
            client,
            redirects: false,
        }
    }
}

#[cfg(feature = "openssl-tls")]
impl Client<hyper_tls::HttpsConnector<HttpConnector>> {
    pub fn new_openssl() -> Self {
        let conn = hyper_tls::HttpsConnector::new();
        let client = hyper::Client::builder().build(conn);
        Self {
            client,
            redirects: false,
        }
    }
}

impl<C> Client<C>
where
    C: Connect + Send + Sync + Clone + 'static + Unpin,
{
    pub fn follow_redirects(&mut self) {
        self.redirects = true;
    }
    async fn check_redirects(&self, url: hyper::Uri) -> Result<hyper::Uri> {
        let req = head!(&url)?;
        let head_req = self.client.request(req.into()).await?;
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
    pub async fn head(&self, url: &str) -> Result<hyper::Response<hyper::Body>> {
        let parsed = url.parse::<hyper::Uri>()?;
        return if self.redirects {
            let new = self.check_redirects(parsed.clone()).await?;
            let req = head!(&new)?;
            self.client
                .request(req.into())
                .await
                .map_err(|e| Error::NetError(e))
        } else {
            let req = head!(&parsed)?;
            self.client
                .request(req.into())
                .await
                .map_err(|e| Error::NetError(e))
        };
    }
    pub async fn get(&self, url: &str) -> Result<Response<Body>> {
        let parsed = url.parse::<hyper::Uri>()?;
        return if self.redirects {
            let new = self.check_redirects(parsed.clone()).await?;
            self.client.get(new).await.map_err(|e| Error::NetError(e))
        } else {
            self.client
                .get(parsed)
                .await
                .map_err(|e| Error::NetError(e))
        };
    }
}

#[async_trait]
pub trait Resp {
    async fn json<T:DeserializeOwned>(self) -> Result<T>;
}

#[async_trait]
impl Resp for Response<Body> {
    async fn json<T>(self) -> Result<T>
    where
        T: DeserializeOwned
    {
        let full = hyper::body::to_bytes(self).await?;
        serde_json::from_slice(&full).map_err(|e| Error::SerError(e))

    }
}