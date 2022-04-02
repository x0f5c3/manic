mod error;
mod tcp;

use crate::error::CodecError;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use bytes::{Bytes, BytesMut};
use chacha20poly1305::Key as ChaChaKey;
use chacha20poly1305::{aead::Aead, aead::NewAead, XChaCha20Poly1305, XNonce};
use error::Result;
use futures::{SinkExt, StreamExt};
use log::debug;
use rand_chacha::ChaCha20Rng;
use manic_proto::{Packet, SymmetricalCodec};
use rand_core::{OsRng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use spake2::{Ed25519Group, Identity, Password};
use std::fmt::Debug;
use std::io;
use std::io::{ErrorKind, Read, Write};
use std::marker::PhantomData;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio_serde::{Deserializer, Framed, Serializer, SymmetricallyFramed};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use zeroize::Zeroize;

#[derive(Deserialize, Serialize, Debug)]
pub struct Key {
    hostname: String,
    ip: String,
    pub key: Vec<u8>,
}

impl Key {
    pub fn new(hostname: String, ip: String, key: Vec<u8>) -> Self {
        Self { hostname, ip, key }
    }
    pub fn generate(hostname: String, ip: String) -> Self {
        let mut key = chacha20poly1305::Key::default();
        let mut rng = rand::rngs::OsRng::default();
        rng.fill_bytes(&mut key);
        Self {
            hostname,
            ip,
            key: key.to_vec(),
        }
    }
}

impl Zeroize for Key {
    fn zeroize(&mut self) {
        self.hostname.zeroize();
        self.key.zeroize();
        self.ip.zeroize();
    }
}

// pub fn start_codec<T>(
//     st: &mut TcpStream,
// ) -> Result<EncryptedBincode<T>> {
// }
pub struct StdConn(TcpStream);

type Writer = manic_proto::Writer<Packet>;

type Reader = manic_proto::Reader<Packet>;

const MAGIC_BYTES: &[u8; 4] = b"croc";

impl StdConn {
    async fn read(&mut self) -> Result<Vec<u8>> {
        let mut header = [0; 4];
        self.0.read(&mut header).await?;
        if &header != MAGIC_BYTES {
            return Err(CodecError::MagicBytes(header))
        }
        header = [0; 4];
        self.0.read(&mut header).await?;
        let data_size: u32 = bincode::deserialize(&header)?;
        let mut buf: Vec<u8> = (0..data_size).into_iter().map(|_| 0).collect();
        self.0.read(&mut buf).await?;
        Ok(buf)
    }
    async fn write(&mut self, buf: &[u8]) -> Result<()> {
        let mut header = MAGIC_BYTES.clone();
        self.0.write(&header).await?;
        let data_size = buf.len() as u32;
        self.0.write(&bincode::serialize(&data_size)?).await?;
        Ok(())
    }
    async fn init_curve_a(mut self, shared: String) -> Result<Conn> {
        let (s, key) = spake2::Spake2::<Ed25519Group>::start_a(
            &Password::new(shared),
            &Identity::new(b"server"),
            &Identity::new(b"client"),
        );
        self.write(&key).await?;
        let bbytes = self.read().await?;
        let strong_key = s.finish(&bbytes)?;
        let pw_hash = new_argon(&strong_key)?;
        self.write(pw_hash.salt.ok_or(CodecError::NOSalt)?.as_bytes())
            .await?;
        Conn::new(self.0, pw_hash.to_string().as_bytes().to_vec())
    }

    async fn init_curve_b(mut self, shared: String) -> Result<Conn> {
        let (s, key) = spake2::Spake2::<Ed25519Group>::start_b(
            &Password::new(shared),
            &Identity::new(b"server"),
            &Identity::new(b"client"),
        );
        let bbytes = self.read().await?;
        let strong_key = s.finish(&bbytes).unwrap();
        self.write(&key).await?;
        let pw_hash = new_argon(&strong_key)?;
        self.write(pw_hash.salt.ok_or(CodecError::NOSalt)?.as_bytes())
            .await?;
        Conn::new(self.0, pw_hash.to_string().as_bytes().to_vec())
    }
}

pub fn new_argon<'a>(pw: &[u8]) -> Result<argon2::PasswordHash<'a>> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw, &salt)
        .map_err(|e| CodecError::from(e))
}

pub struct Conn {
    encoded_send: Writer,
    encoded_recv: Reader,
}

impl Conn {
    pub fn new(conn: TcpStream, key: Vec<u8>) -> Result<Self> {
        let std = conn.into_std()?;
        let write = TcpStream::from_std(std.try_clone()?)?;
        let read = TcpStream::from_std(std)?;
        let len_delim = FramedWrite::new(write, LengthDelimitedCodec::new());

        let mut ser = SymmetricallyFramed::new(
            len_delim,
            SymmetricalCodec::<Packet>::new(key.clone()),
        );
        let len_read = FramedRead::new(read, LengthDelimitedCodec::new());
        let mut de = SymmetricallyFramed::new(len_read, SymmetricalCodec::<Packet>::new(key));
        Ok(Self {
            encoded_send: ser.into(),
            encoded_recv: de.into(),
        })
    }
    pub async fn send(&mut self, packet: Packet) -> Result<()> {
        self.encoded_send.send(packet).await.into()
    }
    pub async fn recv(&mut self) -> Result<Packet> {
        self.encoded_recv.next().await.into()
    }
}
