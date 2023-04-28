use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

mod bincode_codec;

pub const MAGIC_BYTES: &[u8; 5] = b"manic";
#[derive(Debug, Deserialize, Serialize, Encode, Decode)]
pub struct Packet {
    magic: [u8; 5],
    // header: u32,
    data: Vec<u8>,
}
