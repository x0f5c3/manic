use common::time::OffsetDateTime;
use std::sync::Arc;
use tokio::sync::Mutex;

pub enum Peer {
    Discovered(PeerState),
    Lost(PeerState),
}

/// PeerState is the state of a peer that has been discovered.
/// It contains the address of the peer, the last time it was seen,
/// the last payload it sent, and the metadata associated with it.
/// To update the metadata, assign your own metadata to the Metadata.Data field.
pub struct PeerState {
    address: String,
    last_seen: OffsetDateTime,
    last_payload: Vec<u8>,
    metadata: Vec<u8>,
}

