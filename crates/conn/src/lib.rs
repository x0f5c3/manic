#![allow(dead_code)]
mod tcp;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use bincode::config::standard;
use futures::{SinkExt, StreamExt};
use manic_proto::bincode;
use manic_proto::{CodecError, Result};
use manic_proto::{Packet, Reader, Writer};
use rand_core::OsRng;
use spake2::{Ed25519Group, Identity, Password};

use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};

pub struct StdConn(TcpStream);

pub struct Server;

const MAGIC_BYTES: &[u8; 5] = b"manic";

impl StdConn {
    async fn connect<A: Into<SocketAddr>>(addr: A) -> Result<Self> {
        let sock = TcpSocket::new_v4()?;
        sock.set_linger(Some(Duration::from_secs(30)))?;
        Ok(Self(sock.connect(addr.into()).await?))
    }
    async fn read(&mut self) -> Result<Vec<u8>> {
        let mut header = [0; 5];
        self.0.read_exact(&mut header).await?;
        if &header != MAGIC_BYTES {
            return Err(CodecError::MagicBytes(header));
        }
        header = [0; 5];
        self.0.read_exact(&mut header).await?;
        let (data_size, _len): (u32, usize) =
            bincode::decode_from_slice(header.as_slice(), standard())?;
        let mut buf: Vec<u8> = (0..data_size).into_iter().map(|_| 0).collect();
        self.0.read_exact(&mut buf).await?;
        Ok(buf)
    }
    async fn write(&mut self, buf: &[u8]) -> Result<()> {
        let header = *MAGIC_BYTES;
        self.0.write_all(&header).await?;
        let data_size = buf.len() as u32;
        self.0
            .write_all(&bincode::encode_to_vec(&data_size, standard())?)
            .await?;
        self.0
            .write_all(&bincode::encode_to_vec(buf, standard())?)
            .await?;
        Ok(())
    }

    async fn init_curve(mut self, shared: String) -> Result<Conn> {
        let (s, key) = spake2::Spake2::<Ed25519Group>::start_symmetric(
            &Password::new(&shared[5..]),
            &Identity::new((&shared[1..5]).as_ref()),
        );
        let bbytes = self.read().await?;
        let strong_key = s.finish(&bbytes)?;
        self.write(&key).await?;
        let salt = SaltString::generate(&mut OsRng);
        let pw_hash = Argon2::default().hash_password(&strong_key, &salt)?;
        self.write(pw_hash.salt.ok_or(CodecError::NOSalt)?.as_bytes())
            .await?;
        let maybe_salt = self.read().await?;
        if maybe_salt != pw_hash.salt.ok_or(CodecError::NOSalt)?.as_bytes() {
            return Err(CodecError::NOSalt);
        }
        Conn::new(self.0, pw_hash.to_string().as_bytes().to_vec())
    }
}

pub struct Conn {
    encoded_send: Writer,
    encoded_recv: Reader,
}

impl Conn {
    pub fn new(conn: TcpStream, key: Vec<u8>) -> Result<Self> {
        let (read, write) = conn.into_split();
        let ser = Writer::new(write, Some(key.clone()));
        let de = Reader::new(read, Some(key));
        Ok(Self {
            encoded_send: ser,
            encoded_recv: de,
        })
    }
    pub async fn send(&mut self, packet: Packet) -> Result<()> {
        self.encoded_send.send(packet).await?;
        Ok(())
    }
    pub async fn recv(&mut self) -> Result<Packet> {
        let res = self
            .encoded_recv
            .next()
            .await
            .unwrap_or(Err(CodecError::NOSalt));
        res
    }
}
