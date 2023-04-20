#![allow(dead_code)]
extern crate core;

mod codec;
mod error;
mod files;
mod lists;
mod transferinfo;

pub const MAGIC_BYTES: &[u8; 5] = b"manic";

// use crate::files::File;
use serde::{Deserialize, Serialize};

pub use chacha20poly1305;

pub use codec::{Codec, Reader, Writer};

pub use bincode;

use bincode::{Decode, Encode};
pub use codec::{Codec, Reader, Writer};
pub use error::CrocError;
pub use error::{CodecError, Result};

pub use zeroize::Zeroize;
#[derive(Debug, Deserialize, Serialize, Encode, Decode)]
pub struct Packet {
    magic: [u8; 5],
    header: u32,
    data: Vec<u8>,
}

// #[derive(Serialize, Deserialize, Debug)]
// pub struct Packet {
//     header: Header,
//     packet: PacketType,
// }
//
// impl Packet {
//     pub fn new(hostname: String, destination: String, packet: PacketType) -> Self {
//         let header = Header::new(hostname, destination);
//         Self { header, packet }
//     }
//     pub fn get_header(&self) -> &Header {
//         &self.header
//     }
//     pub fn into_packet(self) -> PacketType {
//         self.packet
//     }
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub enum PacketType {
//     File(File),
//     KeyReq,
// }
//
// impl PacketType {
//     pub fn new_file(file: File) -> Self {
//         Self::File(file)
//     }
//     pub fn new_req() -> Self {
//         Self::KeyReq
//     }
// }

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Header {
    pub hostname: String,
    pub destination: String,
    //pub uuid: Uuid,
}

impl Header {
    pub fn new(hostname: String, destination: String) -> Self {
        Self {
            hostname,
            destination,
        }
    }
}

impl Zeroize for Header {
    fn zeroize(&mut self) {
        self.hostname.zeroize();
        self.destination.zeroize();
    }
}
