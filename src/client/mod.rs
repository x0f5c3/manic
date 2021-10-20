mod request;
mod response;
// mod types;

use hyper::header::{CONTENT_LENGTH, LOCATION};
use hyper::{Body, Method, Uri};
#[cfg(feature = "json")]
use serde::de::DeserializeOwned;

use crate::client::request::RequestBuilder;
use crate::ManicError;
use crate::Result;
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use request::Request;
use response::Response;
use tracing::debug;

#[macro_use]
macro_rules! head {
    ($url:expr) => {
        Request::head($url)
            .body(Body::empty())
            .map(|x| x.into::<Request>())
            .map_err(|e| ManicError::REQError(e))
    };
    (&url:ident) => {
        Request::head($url)
            .body(Body::empty())
            .map(|x| x.into::<Request>())
            .map_err(|e| ManicError::REQError(e))
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
    pub fn new() -> Result<Self> {
        let client = hyper::Client::builder()
            .build::<_, Body>(hyper_rustls::HttpsConnector::with_native_roots());
        Ok(Self {
            client,
            redirects: true,
        })
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
    pub async fn request(&self, req: Request) -> Result<Response> {
        let res: Response = self.client.request(req.into()).await?.into();
        if self.redirects {
            let mut check = res.check_redirect(req.uri())?;
            if let Some(new) = check {
                req.change_url(new);
                self.client
                    .request(req.into())
                    .await
                    .map(|x| x.into())
                    .map_err(|e| e.into())
            } else {
                Ok(res)
            }
        } else {
            Ok(res)
        }
    }
    pub fn make_request(&self, method: Method, url: &str) -> Result<RequestBuilder> {
        let req = url.parse::<hyper::Uri>()?;
        Ok(RequestBuilder::new(self.clone(), Request::new(method, req)))
    }
    /// Perform a HEAD request
    pub fn head(&self, url: &str) -> Result<RequestBuilder> {
        self.make_request(Method::HEAD, url)
    }
    /// Perform a GET request
    pub fn get(&self, url: &str) -> Result<RequestBuilder> {
        self.make_request(Method::GET, url)
    }
}
