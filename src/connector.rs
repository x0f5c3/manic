/// Type alias for Rustls connector
#[cfg(feature = "rustls-tls")]
pub type Rustls = hyper_rustls::HttpsConnector<hyper::client::HttpConnector>;
/// Type alias for OpenSSL connector
#[cfg(feature = "openssl-tls")]
pub type OpenSSL = hyper_tls::HttpsConnector<hyper::client::HttpConnector>;

/// Trait implemented for HTTPS connectors
pub trait Connector: Clone + Send + Sync + 'static {
    /// Construct the HttpsConnector
    fn new() -> Self;
}

#[cfg(feature = "rustls-tls")]
impl Connector for Rustls {
    fn new() -> Self {
        hyper_rustls::HttpsConnector::with_native_roots()
    }
}

#[cfg(feature = "openssl-tls")]
impl Connector for OpenSSL {
    fn new() -> Self {
        hyper_tls::HttpsConnector::new()
    }
}
