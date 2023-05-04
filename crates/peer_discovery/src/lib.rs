mod error;
mod listener;

use crate::listener::{Peer, PeerState};
use buildstructor::buildstructor;
use common::time;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// https://en.wikipedia.org/wiki/User_Datagram_Protocol#Packet_structure
///
const MAX_DATAGRAM_SIZE: usize = 65507;

#[derive(Clone, Debug)]
pub struct Notifier {
    inner: Arc<tokio::sync::Notify>,
}

// pub struct DualChannel<T> {
//     receiver: mpsc::Receiver<T>,
//     sender: mpsc::Sender<T>,
// }
//
// impl<T> DualChannel<T> {
//     pub fn new<T>() -> Self {
//         let (sender, receiver) = mpsc::channel::<T>(10);
//         Self { receiver, sender }
//     }
//     pub fn send(&self, item: T) {
//         self.sender.send(item);
//     }
// }

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
    multicast_address: IpAddr,
    /// payload is either the bytes that are sent out with each broadcast
    /// or a function to call every time to generate the payload. Must be short.
    payload: PayloadType<F>,
    // Delay is the amount of time between broadcasts. The default delay is 1 second.
    pub delay: Duration,
    // TimeLimit is the amount of time to spend discovering, if the limit is not reached.
    // A negative limit indiciates scanning until the limit was reached or, if an
    // unlimited scanning was requested, no timeout.
    // The default time limit is 10 seconds.
    pub time_limit: Duration,
    // StopChan is a channel to stop the peer discvoery immediatley after reception.
    stop_channel: mpsc::Receiver<()>,
    pub stop_signal: mpsc::Sender<()>,
    // AllowSelf will allow discovery the local machine (default false)
    pub allow_self: bool,
    // DisableBroadcast will not allow sending out a broadcast
    pub disable_broadcast: bool,
    // Notify will be called each time a new peer was discovered.
    // The default is nil, which means no notification whatsoever.
    pub notify: Option<N>,
    // NotifyLost will be called each time a peer was lost.
    // The default is nil, which means no notification whatsoever.
    // This function should not take too long to execute, as it is called
    // from the peer garbage collector.
    pub notify_lost: Option<NL>,
}

impl<F, N, NL> Default for Settings<F, N, NL>
where
    F: Fn() -> Vec<u8> + Send + Sync + 'static,
    N: Fn(Discovered) + Send + Sync + 'static,
    NL: Fn(PeerState) + Send + Sync + 'static,
{
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel(1);
        Self {
            limit: 0,
            port: 9999,
            multicast_address: IpAddr::from(Ipv4Addr::new(239, 255, 255, 250)),
            payload: PayloadType::Static("hi".as_bytes().to_vec()),
            delay: Duration::from_secs(1),
            time_limit: Duration::from_secs(10),
            stop_channel: receiver,
            stop_signal: sender,
            allow_self: false,
            disable_broadcast: false,
            ..Default::default()
        }
    }
}

#[buildstructor]
impl<F, N, NL> Settings<F, N, NL>
where
    F: Fn() -> Vec<u8> + Send + Sync + 'static,
    N: Fn(Discovered) + Send + Sync + 'static,
    NL: Fn(PeerState) + Send + Sync + 'static,
{
    #[builder(entry = "static_payload")]
    pub fn new_static(
        limit: Option<u64>,
        port: Option<u64>,
        multicast_addr: Option<IpAddr>,
        payload: Vec<u8>,
        maybe_delay: Option<Duration>,
        maybe_limit: Option<Duration>,
        allow_self: bool,
        disable_broadcast: bool,
        notify: Option<N>,
        notify_lost: Option<NL>,
    ) -> Self {
        let multicast_address =
            multicast_addr.unwrap_or(IpAddr::from(Ipv4Addr::new(239, 255, 255, 250)));
        let delay = maybe_delay.unwrap_or(Duration::from_secs(1));
        let time_limit = maybe_limit.unwrap_or(Duration::from_secs(10));
        let (stop_signal, stop_channel) = mpsc::channel(1);
        Self {
            limit: limit.unwrap_or(0),
            port: port.unwrap_or(9999),
            payload: PayloadType::Static(payload),
            multicast_address,
            delay,
            time_limit,
            stop_channel,
            stop_signal,
            allow_self,
            disable_broadcast,
            notify,
            notify_lost,
        }
    }
    #[builder(entry = "dynamic_payload")]
    pub fn new_dynamic(
        limit: Option<u64>,
        port: Option<u64>,
        multicast_addr: Option<IpAddr>,
        payload: F,
        maybe_delay: Option<Duration>,
        maybe_limit: Option<Duration>,
        allow_self: bool,
        disable_broadcast: bool,
        notify: Option<N>,
        notify_lost: Option<NL>,
    ) -> Self {
        let multicast_address =
            multicast_addr.unwrap_or(IpAddr::from(Ipv4Addr::new(239, 255, 255, 250)));
        let delay = maybe_delay.unwrap_or(Duration::from_secs(1));
        let time_limit = maybe_limit.unwrap_or(Duration::from_secs(10));
        let (stop_signal, stop_channel) = mpsc::channel(1);
        Self {
            limit: limit.unwrap_or(0),
            port: port.unwrap_or(9999),
            payload: PayloadType::Dynamic(payload),
            multicast_address,
            delay,
            time_limit,
            stop_channel,
            stop_signal,
            allow_self,
            disable_broadcast,
            notify,
            notify_lost,
        }
    }
}

pub enum PayloadType<F: Fn() -> Vec<u8> + Send + Sync + 'static> {
    Static(Vec<u8>),
    Dynamic(F),
}
