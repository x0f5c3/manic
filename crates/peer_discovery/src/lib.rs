mod listener;

use crate::listener::{Peer, PeerState};
use common::time;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct Notifier {
    inner: Arc<tokio::sync::Notify>,
}

pub struct DualChannel<T> {
    receiver: mpsc::Receiver<T>,
    sender: mpsc::Sender<T>
}


impl<T> DualChannel<T> {
    pub fn new<T>() -> Self {
        let (sender, receiver) = mpsc::channel::<T>(10);
        Self {
            receiver,
            sender
        }
    }
    pub fn send(&self, item: T) {
        self.sender.send(item);
    }
}

enum IPVersion {
    V4,
    V6,
}

pub struct Discovered {
    address: String,
    payload: Vec<u8>,
    metadata: Vec<u8>,
}

/// Settings are the settings that can be specified for
/// doing peer discovery.
pub struct Settings<F, N, NL>
where
    F: Fn() -> Vec<u8> + Send + Sync + 'static,
    N: Fn(Discovered) + Send + Sync + 'static,
    NL: Fn(PeerState) + Send + Sync + 'static,
{
    // Limit is the number of peers to discover, use < 1 for unlimited.
    limit: u64,
    // Port is the port to broadcast on (the peers must also broadcast using the same port).
    // The default port is 9999.
    port: u64,
    // MulticastAddress specifies the multicast address.
    // You should be able to use any of 224.0.0.0/4 or ff00::/8.
    // By default it uses the Simple Service Discovery Protocol
    // address (239.255.255.250 for IPv4 or ff02::c for IPv6).
    multicast_address: String,
    /// payload is the bytes that are sent out with each broadcast. Must be short.
    payload: Vec<u8>,
    /// payload_gen is the function that will be called to dynamically generate payload
    /// before every broadcast. If this pointer is nil `payload` field will be broadcasted instead.
    payload_gen: Option<F>,
    // Delay is the amount of time between broadcasts. The default delay is 1 second.
    pub delay: std::time::Duration,
    // TimeLimit is the amount of time to spend discovering, if the limit is not reached.
    // A negative limit indiciates scanning until the limit was reached or, if an
    // unlimited scanning was requested, no timeout.
    // The default time limit is 10 seconds.
    pub time_limit: std::time::Duration,
    // StopChan is a channel to stop the peer discvoery immediatley after reception.
    pub stop_channel: tokio::sync::mpsc::Receiver<()>,
    // AllowSelf will allow discovery the local machine (default false)
    pub allow_self: bool,
    // DisableBroadcast will not allow sending out a broadcast
    pub disable_broadcast: bool,
    // IPVersion specifies the version of the Internet Protocol (default IPv4)
    pub ip_version: IPVersion,
    // Notify will be called each time a new peer was discovered.
    // The default is nil, which means no notification whatsoever.
    pub notify: Option<N>,
    // NotifyLost will be called each time a peer was lost.
    // The default is nil, which means no notification whatsoever.
    // This function should not take too long to execute, as it is called
    // from the peer garbage collector.
    pub notify_lost: Option<NL>,
    port_num: u16,
    multicast_addr_numbers: std::net::IpAddr,
}
