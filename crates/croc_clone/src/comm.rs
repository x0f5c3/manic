use crate::{CodecError, Packet};
use bytes::{Bytes, BytesMut};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::aead::NewAead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use flate2::Compression;
use futures::{ready, Sink, Stream, TryStream};
use log::debug;
use pin_project_lite::pin_project;
use rand::RngCore;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio_serde::{Deserializer, Serializer};
use tokio_util::codec::length_delimited::LengthDelimitedCodec;
use tokio_util::codec::{FramedRead, FramedWrite};
use zeroize::Zeroize;
use crate::error::CodecError;

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
		let mut res = nonce.to_vec();
		debug!("To serialize: {:?}", item);
		let mut writer = Vec::new();
		let mut parz = flate2::write::DeflateEncoder::new(&mut writer, Compression::best());
		bincode::encode_into_std_write(&item, &mut parz, bincode::config::standard())?;
		let finished = parz.finish()?;
		debug!("Serialized: {:?}", finished);
		res.append(&mut self.cha.encrypt(&nonce, finished.as_slice())?.to_vec());
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
		let res: Packet =
			bincode::decode_from_std_read(&mut decompress, bincode::config::standard())?;
		debug!("Deserialized: {:?}", res);
		Ok(res)
	}
}

pin_project! {
    pub struct Writer {
        #[pin]
        inner: FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        #[pin]
        codec: Codec,
    }
}

pin_project! {
pub struct Reader {
    #[pin]
    inner: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    #[pin]
    codec: Codec,
}
    }

fn gen_key() -> Vec<u8> {
	let mut key = chacha20poly1305::Key::default();
	let mut rng = rand::rngs::OsRng::default();
	rng.fill_bytes(&mut key);
	key.to_vec()
}

impl Writer {
	pub fn new(conn: OwnedWriteHalf, key: Option<Vec<u8>>) -> Self {
		let len_delim = FramedWrite::new(conn, LengthDelimitedCodec::new());

		let codec = Codec::new(key.unwrap_or_else(gen_key));
		Self {
			inner: len_delim,
			codec,
		}
	}
}

impl Reader {
	pub fn new(conn: OwnedReadHalf, key: Option<Vec<u8>>) -> Self {
		let inner = FramedRead::new(conn, LengthDelimitedCodec::new());
		let codec = Codec::new(key.unwrap_or_else(gen_key));

		Self { inner, codec }
	}
}

impl Sink<Packet> for Writer {
	type Error = CodecError;

	fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.project()
			.inner
			.poll_ready(cx)
			.map_err(CodecError::from)
	}

	fn start_send(mut self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
		let res = self.as_mut().project().codec.serialize(&item)?;
		self.project()
			.inner
			.start_send(res)
			.map_err(CodecError::from)
	}

	fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.project()
			.inner
			.poll_flush(cx)
			.map_err(CodecError::from)
	}

	fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.project()
			.inner
			.poll_close(cx)
			.map_err(CodecError::from)
	}
}

impl Stream for Reader {
	type Item = Result<Packet, CodecError>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		match ready!(self.as_mut().project().inner.try_poll_next(cx)) {
			Some(bytes) => Poll::Ready(Some(Ok(self
				.as_mut()
				.project()
				.codec
				.deserialize(&bytes?)?))),
			None => Poll::Ready(None),
		}
	}
}
