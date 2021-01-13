use crate::Error;
use crate::Result;
use hyper::client::connect::Connect;
use hyper::client::HttpConnector;
use hyper::header::{LOCATION, CONTENT_LENGTH};
use hyper::{Body, Request};
#[cfg(feature = "json")]
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

/// Builder pattern for [`Client`][crate::Client]
#[derive(Debug, Clone, Default)]
pub struct ClientBuilder<C>
where
    C: Connect + Send + Sync + Clone,
{
    redirects: bool,
    connector: C,
}

#[cfg(feature = "rustls-tls")]
impl ClientBuilder<hyper_rustls::HttpsConnector<HttpConnector>> {
    /// Construct with rustls
    pub fn rustls() -> Self {
        Self::new(hyper_rustls::HttpsConnector::with_native_roots())
    }
}

#[cfg(feature = "openssl-tls")]
impl ClientBuilder<hyper_tls::HttpsConnector<HttpConnector>> {
    /// Construct with openssl
    pub fn openssl() -> Self {
        Self::new(hyper_tls::HttpsConnector::new())
    }
}

impl<C> ClientBuilder<C>
where
    C: Connect + Send + Sync + Clone,
{
    /// New ClientBuilder with hyper https Connector
    pub fn new(c: C) -> Self {
        Self {
            redirects: false,
            connector: c,
        }
    }
    /// Follow redirects
    pub fn follow_redirects(mut self) -> Self {
        self.redirects = true;
        self
    }
    /// Build the client
    pub fn build(self) -> Client<C> {
        let client = hyper::Client::builder().build::<_, Body>(self.connector);
        Client {
            client,
            redirects: self.redirects,
        }
    }
}

/// Wrapper for [`hyper::Client`][hyper::Client]
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
    /// Construct the client with Rustls connector
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
    /// Construct the client with OpenSSL connector
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
    /// Follow redirects
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
    /// Get the content-length from the url
    pub async fn content_length(&self, url: &str) -> Result<u64> {
        let head = self.head(url).await?;
        head.content_length().await
    }
    /// Make the custom request
    pub async fn request(&self, req: hyper::Request<Body>) -> Result<Response> {
        self.client.request(req).await.map(|e| e.into()).map_err(|e| Error::NetError(e))
    }
    /// Perform a HEAD request
    pub async fn head(&self, url: &str) -> Result<Response> {
        let parsed = url.parse::<hyper::Uri>()?;
        return if self.redirects {
            let new = self.check_redirects(parsed.clone()).await?;
            let req = head!(&new)?;
            self.client
                .request(req.into())
                .await
                .map(|x| x.into())
                .map_err(|e| Error::NetError(e))
        } else {
            let req = head!(&parsed)?;
            self.client
                .request(req.into())
                .await
                .map(|x| x.into())
                .map_err(|e| Error::NetError(e))
        };
    }
    /// Perform a GET request
    pub async fn get(&self, url: &str) -> Result<Response> {
        let parsed = url.parse::<hyper::Uri>()?;
        return if self.redirects {
            let new = self.check_redirects(parsed.clone()).await?;
            self.client.get(new).await.map(|x| x.into()).map_err(|e| Error::NetError(e))
        } else {
            self.client
                .get(parsed)
                .await
                .map(|x| x.into())
                .map_err(|e| Error::NetError(e))
        };
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
    pub async fn content_length(&self) -> Result<u64> {
        let heads = self.0.headers();
        heads[CONTENT_LENGTH]
            .to_str()?
            .parse::<u64>()
            .map_err(Into::into)
    }
    /// Extract text from Response
    pub async fn text(self) -> Result<String> {
        let full = hyper::body::to_bytes(self.0).await?;
        String::from_utf8(full.to_vec()).map_err(|e| Error::UTF8(e))
    }
}

