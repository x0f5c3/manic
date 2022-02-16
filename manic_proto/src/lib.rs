#![allow(dead_code)]
extern crate core;

mod files;
mod lists;

pub use encrypted_bincode::{EncryptedBincode, SymmetricalEncryptedBincode};
pub use manic_rsa::RsaKey;
pub use manic_rsa::{RsaPrivateKey, RsaPublicKey, PADDINGFUNC};

use crate::files::File;
use chacha20poly1305::XChaCha20Poly1305;
use encrypted_bincode::Key;
use rand_core::{OsRng, RngCore};
use rsa::{PaddingScheme, PublicKey};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;
use zeroize::Zeroize;

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
    RSA(RsaPublicKey),
    File(File),
    KeyReq,
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
