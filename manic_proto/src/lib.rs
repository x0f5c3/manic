#![allow(dead_code)]
extern crate core;

mod files;
mod lists;

pub use encrypted_bincode::{EncryptedBincode, SymmetricalEncryptedBincode};
pub use manic_rsa::RsaKey;
pub use manic_rsa::{RsaPrivateKey, RsaPublicKey, PADDINGFUNC};

use crate::files::File;
use chacha20poly1305::XChaCha20Poly1305;
use crc::{Crc, CRC_16_IBM_SDLC};
use encrypted_bincode::Key;
use manic_rsa::RsaPubKey;
use rand_core::{OsRng, RngCore};
use rsa::pkcs1::ToRsaPublicKey;
use rsa::{PaddingScheme, PublicKey};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;
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
    Key(Key),
    RSA(RsaKeyMessage),
    File(File),
    KeyReq,
}

impl PacketType {
    pub fn new_key(key: Key) -> Self {
        Self::Key(key)
    }
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

impl Zeroize for PacketType {
    fn zeroize(&mut self) {
        match self {
            Self::Key(s) => s.zeroize(),
            _ => {}
        }
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

impl Zeroize for Packet {
    fn zeroize(&mut self) {
        self.header.zeroize();
        self.packet.zeroize();
    }
}
