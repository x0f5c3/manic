use crate::error::CodecError;
use crate::{Framed, FramedRead, FramedWrite};
use bytes::{Bytes, BytesMut};
use chacha20poly1305::Key as ChaChaKey;
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use log::debug;
use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::pin::Pin;
use tokio_serde::{Deserializer, Serializer, SymmetricallyFramed};
use zeroize::Zeroize;

pub type SymmetricalCodec<T> = Codec<T, T>;

#[derive(Clone)]
pub struct Codec<Item, SinkItem> {
    key: Vec<u8>,
    cha: XChaCha20Poly1305,
    ghost: PhantomData<(Item, SinkItem)>,
}

impl<I, S> Zeroize for Codec<I, S> {
    fn zeroize(&mut self) {
        self.key.zeroize()
    }
}

impl<I, S> Drop for Codec<I, S> {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl<Item, SinkItem> Codec<Item, SinkItem> {
    pub fn new(key: Vec<u8>) -> Self {
        let cha = XChaCha20Poly1305::new_from_slice(key.as_slice()).unwrap();
        Self {
            cha,
            key,
            ghost: PhantomData::default(),
        }
    }
}

impl<Item, SinkItem> Serializer<SinkItem> for Codec<Item, SinkItem>
where
    SinkItem: Serialize + Debug,
{
    type Error = CodecError;

    fn serialize(self: Pin<&mut Self>, item: &SinkItem) -> Result<Bytes, Self::Error> {
        let mut nonce = XNonce::default();
        let mut rng = ChaCha20Rng::from_entropy();
        rng.fill_bytes(&mut nonce);
        let key = ChaChaKey::from_slice(self.key.as_slice());
        let cipher = XChaCha20Poly1305::new(key);
        let mut res = nonce.to_vec();
        debug!("To serialize: {:?}", item);
        let ser = bincode::serialize(&item)?;
        let mut writer = Vec::new();
        let mut parz = flate2::write::DeflateEncoder::new(&mut writer, Compression::best());
        parz.write_all(&ser)?;
        parz.finish()?;
        debug!("Serialized: {:?}", ser);
        res.append(&mut self.cha.encrypt(&nonce, writer.as_slice())?.to_vec());
        debug!("Encrypted: {:?}", res);
        Ok(Bytes::from(res))
    }
}

impl<Item, SinkItem> Deserializer<Item> for Codec<Item, SinkItem>
where
    Item: Debug,
    for<'a> Item: Deserialize<'a>,
{
    type Error = CodecError;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Item, Self::Error> {
        if src.len() < 25 {
            return Err(CodecError::TooShort(src.len()));
        }
        debug!("To decrypt: {:?}", src);
        let dec = self
            .cha
            .decrypt(XNonce::from_slice(&src[..24]), &src[24..])?;
        debug!("Decrypted: {:?}", dec);
        let mut decompress = flate2::read::DeflateDecoder::new(dec.as_slice());
        let mut decompressed = Vec::new();
        decompress.read_to_end(&mut decompressed)?;
        let res: Item = bincode::deserialize(decompressed.as_slice())?;
        debug!("Deserialized: {:?}", res);
        Ok(res)
    }
}

use crate::LengthDelimitedCodec;
use crate::Packet;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::aead::NewAead;
use flate2::Compression;

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

pub struct Writer(
    Framed<
        FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        Packet,
        Packet,
        SymmetricalCodec<Packet>,
    >,
);
pub struct Reader(
    Framed<
        FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        Packet,
        Packet,
        SymmetricalCodec<Packet>,
    >,
);

fn gen_key() -> Vec<u8> {
    let mut key = chacha20poly1305::Key::default();
    let mut rng = rand::rngs::OsRng::default();
    rng.fill_bytes(&mut key);
    key.to_vec()
}

impl Writer {
    pub fn new(conn: OwnedWriteHalf, key: Option<Vec<u8>>) -> Self {
        let len_delim = FramedWrite::new(conn, LengthDelimitedCodec::new());

        let ser: Framed<
            FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
            Packet,
            Packet,
            SymmetricalCodec<Packet>,
        > = SymmetricallyFramed::new(
            len_delim,
            SymmetricalCodec::<Packet>::new(key.unwrap_or_else(|| gen_key())),
        );
        Self(ser)
    }
}

impl Reader {
    pub fn new(conn: OwnedReadHalf, key: Option<Vec<u8>>) -> Self {
        let len_delim = FramedRead::new(conn, LengthDelimitedCodec::new());

        let ser: Framed<
            FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
            Packet,
            Packet,
            SymmetricalCodec<Packet>,
        > = SymmetricallyFramed::new(
            len_delim,
            SymmetricalCodec::<Packet>::new(key.unwrap_or_else(|| gen_key())),
        );
        Self(ser)
    }
}
