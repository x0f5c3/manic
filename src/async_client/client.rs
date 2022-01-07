use crate::{ManicError, Result};
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use hyper::body::HttpBody;
use hyper::client::HttpConnector;
use hyper::header::HeaderName;
use hyper::header::CONTENT_LENGTH;
use hyper::header::LOCATION;
use hyper::http::HeaderValue;
use hyper::{Body, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
#[cfg(feature = "json")]
use serde::de::DeserializeOwned;
use std::borrow::BorrowMut;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Client {
    hyper: hyper::Client<HttpsConnector<HttpConnector>>,
    redirects: bool,
}

pub struct Url {
    uri: hyper::Uri,
    url: url::Url,
}

#[derive(Copy, Clone, Debug)]
pub enum ReqType {
    GET,
    HEAD,
}

pub struct Request(http::request::Builder);

impl Request {
    pub fn header(&mut self, key: HeaderName, value: HeaderValue) -> &mut Self {
        if let Some(map) = self.0.headers_mut() {
            map.insert(key, value);
        }
        self
    }
    pub fn build(self) -> Result<hyper::Request<Body>> {
        self.0.body(Body::empty()).map_err(ManicError::HyperHttpErr)
    }
}

impl ReqType {
    pub fn make_req(
        &self,
        url: &Uri,
        headers: Option<Vec<(HeaderName, HeaderValue)>>,
    ) -> Result<hyper::Request<Body>> {
        match self {
            Self::GET => {
                let mut req = hyper::Request::get(url);
                if let Some(map) = req.headers_mut() {
                    if let Some(heads) = headers {
                        for (k, v) in heads {
                            map.insert(k, v.clone());
                        }
                    }
                }
                req.body(Body::empty()).map_err(ManicError::HyperHttpErr)
            }
            Self::HEAD => {
                let mut req = hyper::Request::head(url);
                if let Some(map) = req.headers_mut() {
                    if let Some(heads) = headers {
                        for (k, v) in heads.iter() {
                            map.insert(k, v.clone());
                        }
                    }
                }
                req.body(Body::empty()).map_err(ManicError::HyperHttpErr)
            }
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Client::new(true)
    }
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
    pub async fn request(
        &self,
        url: String,
        typ: ReqType,
        headers: Option<Vec<(HeaderName, HeaderValue)>>,
    ) -> Result<Response> {
        let mut url1 = hyper::Uri::from_str(&url)?;
        let mut req = typ.make_req(&url1, headers.clone())?;
        let mut resp = self.hyper.request(req).await?;
        let mut res = Response::from_hyper(resp.into(), url1.clone());
        if self.redirects {
            while let Some(u) = res.check_redirect()? {
                url1 = u.clone();
                req = typ.make_req(&url1, headers.clone())?;
                resp = self.hyper.request(req).await?;
                res = Response::from_hyper(resp, u);
            }
        }
        Ok(res)
    }
    pub async fn get(
        &self,
        url: String,
        headers: Option<Vec<(HeaderName, HeaderValue)>>,
    ) -> Result<Response> {
        // let mut url = hyper::Uri::from_str(&url)?;
        // let mut resp = &self.hyper.get(url.clone()).await?;
        // let mut res = Response::from_hyper(resp, url.clone());
        // if self.redirects {
        //     while let Some(u) = res.check_redirect()? {
        //         url = u.clone();
        //         resp = &self.hyper.get(u.clone()).await?;
        //         res = Response::from_hyper(resp, u);
        //     }
        // }
        // Ok(Response::from_hyper(&resp, url))
        self.request(url, ReqType::GET, headers).await
    }
    pub async fn head(
        &self,
        url: String,
        headers: Option<Vec<(HeaderName, HeaderValue)>>,
    ) -> Result<Response> {
        self.request(url, ReqType::HEAD, headers).await
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
        let full = hyper::body::to_bytes(self.resp.into_body()).await?;
        serde_json::from_slice(&full).map_err(ManicError::JSONErr)
    }
    /// Get the content-length from Response
    pub fn content_length(&self) -> Option<u64> {
        // let heads = self.resp.headers();
        self.resp
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|x| x.to_str().ok())
            .and_then(|x| x.parse::<u64>().ok())
            .or_else(|| self.resp.body().size_hint().exact())
        // if let Some(l) = len.parse::<u64>().ok() {
        //     Some(l)
        // } else {
        //     self.resp.body().size_hint().exact().ok_or(ManicError::NoLen)
        // }
    }
    pub async fn chunk(&mut self) -> Result<Option<Bytes>> {
        if let Some(item) = self.resp.body_mut().next().await {
            Ok(Some(item?))
        } else {
            Ok(None)
        }
    }
    pub fn bytes_stream(self) -> impl futures::stream::Stream<Item = Result<Bytes>> {
        futures::TryStreamExt::map_err(self.resp.into_body(), ManicError::HyperErr)
    }
    /// Extract text from Response
    pub async fn text(self) -> Result<String> {
        let full = hyper::body::to_bytes(self.resp.into_body()).await?;
        String::from_utf8(full.to_vec()).map_err(ManicError::UTF8)
    }
    /// Check if the response redirects
    pub fn check_redirect(&self) -> Result<Option<hyper::Uri>> {
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
                let cloned = self.url.clone();
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
