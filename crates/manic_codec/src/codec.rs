use crate::error::CodecError;
use bytes::{Bytes, BytesMut};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::aead::NewAead;
use chacha20poly1305::Key as ChaChaKey;
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use flate2::Compression;
use futures::{ready, Sink, Stream};
use log::debug;
use manic_proto::Packet;
use pin_project::pin_project;
use rand::RngCore;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use std::io::{Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio_serde::{Deserializer, Framed, Serializer, SymmetricallyFramed};
use tokio_util::codec::length_delimited::LengthDelimitedCodec;
use tokio_util::codec::{FramedRead, FramedWrite};
use zeroize::Zeroize;

#[derive(Clone)]
pub struct Codec {
    key: Vec<u8>,
    cha: XChaCha20Poly1305,
}

impl Zeroize for Codec {
    fn zeroize(&mut self) {
        self.key.zeroize()
    }
}

impl Drop for Codec {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl Codec {
    pub fn new(key: Vec<u8>) -> Self {
        let cha = XChaCha20Poly1305::new_from_slice(key.as_slice()).unwrap();
        Self { cha, key }
    }
}

impl Serializer<Packet> for Codec {
    type Error = CodecError;

    fn serialize(self: Pin<&mut Self>, item: &Packet) -> Result<Bytes, Self::Error> {
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
        let finished = parz.finish()?;
        debug!("Serialized: {:?}", ser);
        res.append(&mut self.cha.encrypt(&nonce, writer.as_slice())?.to_vec());
        debug!("Encrypted: {:?}", res);
        Ok(Bytes::from(res))
    }
}

impl Deserializer<Packet> for Codec {
    type Error = CodecError;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Packet, Self::Error> {
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
        let res: Packet = bincode::deserialize(decompressed.as_slice())?;
        debug!("Deserialized: {:?}", res);
        Ok(res)
    }
}

#[pin_project]
pub struct Writer(
    #[pin] Framed<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>, Packet, Packet, Codec>,
);
#[pin_project]
pub struct Reader(Framed<FramedRead<OwnedReadHalf, LengthDelimitedCodec>, Packet, Packet, Codec>);

fn gen_key() -> Vec<u8> {
    let mut key = chacha20poly1305::Key::default();
    let mut rng = rand::rngs::OsRng::default();
    rng.fill_bytes(&mut key);
    key.to_vec()
}

impl Writer {
    pub fn new(conn: OwnedWriteHalf, key: Option<Vec<u8>>) -> Self {
        let len_delim = FramedWrite::new(conn, LengthDelimitedCodec::new());

        let ser: Framed<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>, Packet, Packet, Codec> =
            SymmetricallyFramed::new(len_delim, Codec::new(key.unwrap_or_else(|| gen_key())));
        Self(ser)
    }
}

impl Reader {
    pub fn new(conn: OwnedReadHalf, key: Option<Vec<u8>>) -> Self {
        let len_delim = FramedRead::new(conn, LengthDelimitedCodec::new());

        let ser: Framed<FramedRead<OwnedReadHalf, LengthDelimitedCodec>, Packet, Packet, Codec> =
            SymmetricallyFramed::new(len_delim, Codec::new(key.unwrap_or_else(|| gen_key())));
        Self(ser)
    }
}

impl AsMut<Framed<FramedRead<OwnedReadHalf, LengthDelimitedCodec>, Packet, Packet, Codec>>
    for Reader
{
    fn as_mut(
        &mut self,
    ) -> &mut Framed<FramedRead<OwnedReadHalf, LengthDelimitedCodec>, Packet, Packet, Codec> {
        &mut self.0
    }
}

impl Sink<Packet> for Writer {
    type Error = CodecError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().0.poll_ready(cx).map_err(CodecError::from)
    }

    fn start_send(self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        self.project().0.start_send(item).map_err(CodecError::from)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().0.poll_flush(cx).map_err(CodecError::from)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().0.poll_close(cx).map_err(CodecError::from)
    }
}

impl Stream for Reader {
    type Item = Result<Packet, CodecError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.as_mut().0)
            .poll_next(cx)
            .map_err(CodecError::from)
    }
}
