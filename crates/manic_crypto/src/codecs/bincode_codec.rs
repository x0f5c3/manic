use super::Packet;
use crate::CryptoError;
use chacha20poly1305::aead::Aead;
use tracing::debug;

use bytes::{Bytes, BytesMut};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use flate2::Compression;
use futures::{ready, Sink, Stream, TryStream};
use pin_project_lite::pin_project;
use rand_core::{OsRng, RngCore};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio_serde::{Deserializer, Framed, Serializer};
use tokio_util::codec::length_delimited::LengthDelimitedCodec;
use tokio_util::codec::{FramedRead, FramedWrite};
use zeroize::Zeroize;

pub type BincodeFramed = Framed<TcpStream, Packet, Packet, BincodeCodec>;

#[derive(Clone)]
pub struct BincodeCodec {
    key: Vec<u8>,
    cha: XChaCha20Poly1305,
}

impl Zeroize for BincodeCodec {
    fn zeroize(&mut self) {
        self.key.zeroize()
    }
}

impl Drop for BincodeCodec {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl BincodeCodec {
    pub fn new(key: Vec<u8>) -> Self {
        let cha = XChaCha20Poly1305::new_from_slice(key.as_slice()).unwrap();
        Self { cha, key }
    }
}

fn new_nonce(rng: &mut OsRng) -> XNonce {
    let mut nonce = XNonce::default();
    rng.fill_bytes(&mut nonce);
    nonce
}

impl Serializer<Packet> for BincodeCodec {
    type Error = CryptoError;

    fn serialize(self: Pin<&mut Self>, item: &Packet) -> Result<Bytes, Self::Error> {
        let nonce = new_nonce(&mut OsRng);
        let mut res = nonce.to_vec();
        debug!("To serialize: {:?}", item);
        let mut writer = Vec::new();
        let mut parz = flate2::write::DeflateEncoder::new(&mut writer, Compression::best());
        bincode::encode_into_std_write(item, &mut parz, bincode::config::standard())?;
        let finished = parz.finish()?;
        debug!("Serialized: {:?}", finished);
        res.append(&mut self.cha.encrypt(&nonce, finished.as_slice())?.to_vec());
        debug!("Encrypted: {:?}", res);
        Ok(Bytes::from(res))
    }
}

impl Deserializer<Packet> for BincodeCodec {
    type Error = CryptoError;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Packet, Self::Error> {
        if src.len() < 25 {
            return Err(CryptoError::TooShort(src.len()));
        }
        debug!("To decrypt: {:?}", src);
        let dec = self
            .cha
            .decrypt(XNonce::from_slice(&src[..24]), &src[24..])?;
        debug!("Decrypted: {:?}", dec);
        let mut decompress = flate2::read::DeflateDecoder::new(dec.as_slice());
        let res: Packet =
            bincode::decode_from_std_read(&mut decompress, bincode::config::standard())?;
        debug!("Deserialized: {:?}", res);
        Ok(res)
    }
}
