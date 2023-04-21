use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Notifier {
    inner: Arc<tokio::sync::Notify>,
}

enum IPVersion {
    V4,
    V6,
}

pub struct Discovered {
    address: String,
    payload: Vec<u8>,
}

pub struct Settings<F: Fn() -> Vec<u8>> {
    limit: u64,
    port: u64,
    multicast_address: String,
    payload: Vec<u8>,
    payload_gen: Option<F>,
    delay: std::time::Duration,
    time_limit: std::time::Duration,
}
