use crate::Result;
use hyper::client::HttpConnector;
use hyper::service::Service;
use hyper::{Body, Uri};
use std::ops::{Deref, DerefMut};

type HyperClient<T: Service<Uri>> = hyper::Client<T>;

#[derive(Clone, Debug)]
pub(crate) enum ClientBuilder {
    #[cfg(feature = "rustls-tls")]
    RUSTLS,
    #[cfg(feature = "openssl-tls")]
    OPENSSL,
}

impl ClientBuilder {
    pub fn build(&self) -> Result<HyperClient<impl Service<Uri>>> {
        let build = hyper::Client::builder();
        match self {
            #[cfg(feature = "rustls-tls")]
            Self::RUSTLS => {
                Ok(build.build::<_, Body>(hyper_rustls::HttpsConnector::with_native_roots()))
            }
            #[cfg(feature = "openssl-tls")]
            Self::OPENSSL => Ok(build.build::<_, Body>(hyper_openssl::HttpsConnector::new()?)),
        }
    }
    #[cfg(feature = "rustls-tls")]
    pub fn default() -> Result<HyperClient<impl Service<Uri>>> {
        Ok(HyperClient::builder().build(hyper_rustls::HttpsConnector::with_native_roots()))
    }
    #[cfg(all(not(feature = "rustls-tls")), feature = "openssl-tls")]
    pub fn default() -> Result<HyperClient<impl Service<Uri>>> {
        Ok(HyperClient::builder().build(hyper_openssl::HttpsConnector::new()?))
    }
}
