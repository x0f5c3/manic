use hyper::client::HttpConnector;
use hyper::service::Service;
use hyper::{Body, Uri};
use std::ops::{Deref, DerefMut};
use crate::Result;

type HyperClient<T: Service<Uri>> = hyper::Client<T>;

#[derive(Clone, Debug)]
pub(crate) enum HttpClients {
    #[cfg(feature = "rustls-tls")]
    RUSTLS(HyperClient<hyper_rustls::HttpsConnector<HttpConnector>>),
    #[cfg(feature = "openssl-tls")]
    OPENSSL(HyperClient<hyper_openssl::HttpsConnector<HttpConnector>>),
}


impl HttpClients {
    #[cfg(feature = "rustls-tls")]
    pub(crate) fn new() -> Result<Self> {
        Ok(Self::new_rustls())
    }
    #[cfg(not(feature = "rustls-tls"), feature = "openssl-tls")]
    fn new() -> Result<Self> {
        Self::new_openssl()
    }
    #[cfg(feature = "rustls-tls")]
    fn new_rustls() -> Self {
        Self::RUSTLS(hyper::Client::builder().build::<_, Body>(hyper_rustls::HttpsConnector::with_native_roots()))
    }
    #[cfg(feature = "openssl-tls")]
    fn new_openssl() -> Result<Self> {
        Ok(Self::OPENSSL(hyper::Client::builder().build::<_, Body>(hyper_openssl::HttpsConnector::new()?)))
    }
}

impl Deref for HttpClients {
    type Target = HyperClient<T>;
    fn deref(&self) -> &Self::Target {
        match self {
            #[cfg(feature = "rustls-tls")]
            Self::RUSTLS(conn) => &conn,
            #[cfg(feature = "openssl-tls")]
            Self::OPENSSL(conn) => &conn,
        }
    }
}

impl DerefMut for HttpClients {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::RUSTLS(mut conn) => &mut conn,
            Self::OPENSSL(mut conn) => &mut conn,
        }
    }
}
