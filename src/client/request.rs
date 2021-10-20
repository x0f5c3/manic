use crate::client::response::Response;
use crate::header::{HeaderMap, HeaderName, HeaderValue};
use crate::http;
use crate::{Client, ManicError, Result};
use hyper::Method;
use hyper::{Body, Uri};
use std::convert::TryFrom;

#[derive(Debug)]
pub struct Request(hyper::Request<Body>);

impl From<hyper::Request<Body>> for Request {
    fn from(r: http::Request<Body>) -> Self {
        Self(r)
    }
}

impl From<hyper::Request<()>> for Request {
    fn from(r: hyper::Request<()>) -> Self {
        let (parts, _) = r.into_parts();
        let req = hyper::Request::<Body>::from_parts(parts, Body::empty());
        Self(req)
    }
}

impl Request {
    pub fn new(method: Method, url: Uri) -> Result<Self> {
        hyper::Request::builder()
            .method(method)
            .uri(url)
            .body(())
            .map(|x| x.into())
            .map_err(|e| e.into())
    }
    pub fn change_url(mut self, new: Uri) -> Self {
        let mut uri = self.0.uri_mut();
        uri = &mut new.clone();
        self
    }
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.0.headers_mut()
    }
}

#[derive(Debug)]
pub struct RequestBuilder {
    client: Client,
    request: Result<Request>,
}

impl RequestBuilder {
    pub fn new(client: Client, request: Result<Request>) -> RequestBuilder {
        RequestBuilder { client, request }
    }
    pub fn header<K, V>(mut self, key: K, value: V) -> RequestBuilder
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<ManicError>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<ManicError>,
    {
        let mut error: Option<ManicError> = None;
        if let Ok(ref mut req) = self.request {
            match <HeaderName as TryFrom<K>>::try_from(key) {
                Ok(key) => match <HeaderValue as TryFrom<V>>::try_from(value) {
                    Ok(mut value) => {
                        req.headers_mut().append(key, value);
                    }
                    Err(e) => error = Some(e.into()),
                },
                Err(e) => error = Some(e.into()),
            };
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }
    pub async fn send(self) -> Result<Response> {
        match self.request {
            Ok(req) => self.client.request(req).await,
            Err(err) => Err(err),
        }
    }
}
//
// impl RequestBuilder {
//     fn new<T>(client: HttpClients, uri: T) -> Self
//     where
//         Uri: TryFrom<T>,
//         <Uri as TryFrom<T>>::Error: Into<ManicError>,
//     {
//         let builder = hyper::Request::builder().uri(uri);
//         Self {
//             client,
//             req: None,
//             builder,
//         }
//     }
//     pub fn headers(self, heads: &HeaderMap<HeaderValue>) -> RequestBuilder {
//         let mut build = self.builder;
//         heads
//             .iter()
//             .par_bridge()
//             .for_each(|(k, v)| build = self.builder.header(k, v));
//         Self {
//             client: self.client,
//             req: self.req,
//             builder: build,
//         }
//     }
//     pub fn build(&self, body: Body) -> Result<Request<Body>> {
//         self.builder.body(body).map_err(|e| e.into())
//     }
// }
