#![allow(dead_code)]
extern crate core;

mod codec;
mod error;
mod files;
mod lists;

pub use tokio_serde::{Framed, SymmetricallyFramed};
pub use tokio_util::codec::{length_delimited::LengthDelimitedCodec, FramedRead, FramedWrite};

pub use codec::{Codec, Reader, SymmetricalCodec, Writer};
pub use error::CodecError;

use crate::files::File;

use crc::{Crc, CRC_16_IBM_SDLC};

use serde::{Deserialize, Serialize};

use zeroize::Zeroize;

#[derive(Serialize, Deserialize, Debug)]
pub struct RsaKeyMessage {
    crc: u16,
    pub data: Vec<u8>,
}

impl RsaKeyMessage {
    pub fn new(data: Vec<u8>) -> Self {
        let crc = Crc::<u16>::new(&CRC_16_IBM_SDLC).checksum(&data);
        Self { crc, data }
    }
    pub fn check_crc(&self) -> bool {
        let sum = Crc::<u16>::new(&CRC_16_IBM_SDLC).checksum(&self.data);
        sum == self.crc
    }
}

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
    RSA(RsaKeyMessage),
    File(File),
    KeyReq,
}

impl PacketType {
    pub fn new_rsa(key: Vec<u8>) -> Self {
        Self::RSA(RsaKeyMessage::new(key))
    }
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
