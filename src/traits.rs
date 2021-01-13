/// Trait implemented for HTTPS connectors
pub trait Connector {
    /// Construct the HttpsConnector
    fn new() -> Self;
}

#[cfg(feature = "rustls-tls")]
impl Connector for crate::Rustls {
    fn new() -> Self {
        hyper_rustls::HttpsConnector::with_native_roots()
    }
}

#[cfg(feature = "openssl-tls")]
impl Connector for crate::OpenSSL {
    fn new() -> Self {
        hyper_tls::HttpsConnector::new()
    }
}
