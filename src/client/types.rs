use hyper::client::HttpConnector;
use hyper::service::Service;
use hyper::Uri;
use std::ops::{Deref, DerefMut};

type HyperClient<T: Service<Uri>> = hyper::Client<T>;

pub(crate) enum HttpClients {
    #[cfg(feature = "rustls-tls")]
    RUSTLS(HyperClient<hyper_rustls::HttpsConnector<HttpConnector>>),
    #[cfg(feature = "openssl-tls")]
    OPENSSL(HyperClient<hyper_openssl::HttpsConnector<HttpConnector>>),
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
