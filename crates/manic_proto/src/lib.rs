#![allow(dead_code)]
extern crate core;

mod codec;
mod error;
mod files;
mod lists;

pub use tokio_serde::formats::*;
pub use tokio_serde::{Framed, SymmetricallyFramed};
pub use tokio_util::codec::{length_delimited::LengthDelimitedCodec, FramedRead, FramedWrite};

pub use codec::{Codec, Reader, SymmetricalCodec, Writer};
pub use error::CodecError;

use crate::files::File;

use crc::{Crc, CRC_16_IBM_SDLC};

use serde::{Deserialize, Serialize};

pub use argon2;
pub use chacha20poly1305;

pub use zeroize::Zeroize;

#[derive(Serialize, Deserialize, Debug)]
pub struct Packet {
    header: Header,
    packet: PacketType,
}

impl Packet {
    pub fn new(hostname: String, destination: String, packet: PacketType) -> Self {
        let header = Header::new(hostname, destination);
        Self { header, packet }
    }
    pub fn get_header(&self) -> &Header {
        &self.header
    }
    pub fn into_packet(self) -> PacketType {
        self.packet
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum PacketType {
    File(File),
    KeyReq,
}

impl PacketType {
    pub fn new_file(file: File) -> Self {
        Self::File(file)
    }
    pub fn new_req() -> Self {
        Self::KeyReq
    }
}

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