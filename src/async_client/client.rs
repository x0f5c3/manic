use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper::client::HttpConnector;



pub(crate) struct Client {
	hyper: hyper::Client<HttpsConnector<HttpConnector>>,
	redirects: bool,
}



impl Client {
	pub fn new(redirect: bool) -> Self {
		let conn = HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().build();
		let hyp = hyper::Client::builder().build(conn);
		Self {
			hyper: hyp,
			redirects: redirect,
		}
	}
}