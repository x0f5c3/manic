use anyhow::{anyhow, Context, Result};
use futures::sink::SinkExt;
use futures::StreamExt;
use manic_proto::PacketType::Key;
use manic_proto::{
    ChaCha20Rng, EncryptedBincode, Header, RsaPublicKey, SymmetricalEncryptedBincode,
};
use manic_proto::{ChaChaKey, Packet, PacketType, PADDINGFUNC};
use manic_proto::{Key, RsaKey};
use manic_rsa::{PublicKey, RsaPrivKey, RsaPubKey};
use rand_core::{OsRng, RngCore, SeedableRng};
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use spake2::{Ed25519Group, Identity, Password};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio_serde::formats::{Bincode, SymmetricalBincode};
use tokio_serde::{Framed, SymmetricallyFramed};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

const RSAMSGLEN: usize = 512;


const WEAK_KEY: [u8; 3] = [1, 2, 3];

type Writer = Framed<
    FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    Packet,
    Packet,
    EncryptedBincode<Packet, Packet>,
>;

type Reader = Framed<
    FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
    Packet,
    Packet,
    EncryptedBincode<Packet, Packet>,
>;

const MAGIC_BYTES: &[u8; 4] = b"croc";

pub trait Net {}


pub struct StdConn(TcpStream);


impl StdConn {
    async fn read(&mut self) -> Result<Vec<u8>> {
        let mut header = [0; 4];
        self.0.read(&mut header).await?;
        if &header != MAGIC_BYTES {
            anyhow::anyhow!("Magic is wrong")
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
        self.write(pw_hash.salt.context("No salt")?.as_bytes()).await?;
        Conn::new(self.0, pw_hash.to_string().as_bytes().to_vec())
    }

    async fn init_curve_b(mut self, shared: String) -> Result<Conn> {
        let (s, key) = spake2::Spake2::<Ed25519Group>::start_b(
            &Password::new(shared),
            &Identity::new(b"server"),
            &Identity::new(b"client"),
        );
        let bbytes = self.read().unwrap();
        let strong_key = s.finish(&bbytes).unwrap();
        self.write(&key).unwrap();
        let pw_hash = new_argon(&strong_key)?;
        self.write(pw_hash.salt.context("No salt")?.as_bytes()).await?;
        Conn::new(self.0, pw_hash.to_string().as_bytes().to_vec())
    }
}

pub fn new_argon<'a>(pw: &[u8]) -> Result<argon2::PasswordHash<'a>> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default().hash_password(pw, &salt).context("Failed to hash the password")
}

impl Net for StdConn {}

impl Net for TcpStream {}

impl Net for Conn {}

pub struct Conn {
    encoded_send: Writer,
    encoded_recv: Reader,
}

impl Conn {
    pub fn new(conn: TcpStream, key: Vec<u8>) -> Result<Self> {
        let (read, write) = conn.into_split();
        let len_delim = FramedWrite::new(write, LengthDelimitedCodec::new());

        let mut ser = SymmetricallyFramed::new(
            len_delim,
            SymmetricalEncryptedBincode::<Packet>::new(key.clone()),
        );
        let len_read = FramedRead::new(read, LengthDelimitedCodec::new());
        let mut de =
            SymmetricallyFramed::new(len_read, SymmetricalEncryptedBincode::<Packet>::new(key));
        Ok(Self {
            encoded_send: ser.into(),
            encoded_recv: de.into(),
        })
    }
    pub async fn send(&mut self, packet: Packet) -> Result<()> {
        self.encoded_send
            .send(packet)
            .await
            .context("Failed to send packet")
    }
    pub async fn recv(&mut self) -> Result<Packet> {
        self.encoded_recv
            .next()
            .await
            .context("Failed to receive")?
            .context("Failed to parse")
    }
}

pub struct Server<C: Net> {
    rsa: RsaKey,
    hostname: String,
    remote_addr: String,
    conn: C,
}

impl Server<TcpStream> {
    pub async fn new(mut conn: TcpStream) -> Result<Self> {
        let mut to_recv = [0; 1024];
        let msg_len = conn.read(&mut to_recv).await?;
        let to_recv = to_recv.into_iter().take(msg_len).collect();
        if to_recv.len() != msg_len {
            anyhow!("Msg not received fully")
        }
        let client_key: Packet = bincode::deserialize(&to_recv)?;
        if let PacketType::RSA(msg) = client_key {
            if msg.check_crc() {
                let mut key = RsaKey::new(None)?;
                key.peer_key = key
                    .decrypt(&msg.data)
                    .ok()
                    .and_then(|x| x.to_str().ok())
                    .and_then(|x| RsaPublicKey::from_pkcs1_pem(x).ok().into());
                let hostname = hostname::get()?.to_str().unwrap().to_string();
                let to_send = Packet::new(
                    hostname.clone(),
                    conn.peer_addr()?.to_string(),
                    PacketType::new_rsa(key.prep_send()?)?.clone(),
                );
                let to_send = bincode::serialize(&to_send)?;
                conn.write_all(&to_send).await?;
                Ok(Self {
                    rsa: key,
                    hostname,
                    remote_addr: conn.peer_addr()?.to_string(),
                    conn,
                })
            } else {
                anyhow!("CRC check failed")
            }
        } else {
            anyhow!("Wrong packet type")
        }
    }
    pub async fn send_key(mut self) -> Result<Server<Conn>> {
        let key = Key::generate(self.hostname, self.remote_addr.clone());
        let mut to_recv = [0; RSAMSGLEN];
        self.conn.read(&mut to_recv);
        let dec: Packet = bincode::deserialize(&self.rsa.decrypt(&to_recv)?)?;
        if dec.into_packet() == PacketType::KeyReq {
            let to_send = Packet::new(
                self.hostname.clone(),
                self.remote_addr.clone(),
                PacketType::Key(key),
            );
            let enc = self.rsa.encrypt(&bincode::serialize(&to_send)?)?;
            self.conn.write(&enc).await?;
            let conn = Conn::new(self.conn, key.key.clone())?;
            Ok(Server {
                hostname,
                rsa: self.rsa,
                remote_addr: self.remote_addr,
                conn,
            })
        } else {
            Err(anyhow!("Expected msg KEY"))
        }
    }
}

pub struct Client<C: Net> {
    rsa: RsaKey,
    hostname: String,
    remote_addr: String,
    conn: C,
}

impl Client<TcpStream> {
    pub async fn new(url: String) -> Result<Self> {
        let mut conn = TcpStream::connect(&url).await?;
        let priv_key = RsaPrivKey::new()?;
        let mut pub_key = RsaKey::new_from_priv(priv_key, None)?;
        let hostname = hostname::get()?.to_str().unwrap().to_string();
        let to_send = bincode::serialize(&Packet::new(
            hostname.clone(),
            url.clone(),
            PacketType::new_rsa(pub_key.prep_send()?),
        ))?;
        conn.write_all(&to_send).await?;
        let mut recv_key = [0; 2048];
        let msg_len = conn.read(&mut recv_key).await?;
        let recv_key = recv_key.into_iter().take(msg_len).collect();
        if recv_key.len() != msg_len {
            anyhow!("Message not received fully")
        }
        let host_key: Packet = bincode::deserialize(&key.decrypt(&recv_key)?)?;
        if let PacketType::RSA(key_msg) = host_key {
            if key_msg.check_crc() {
                pub_key.peer_key = pub_key
                    .decrypt(&key_msg.data)
                    .ok()
                    .and_then(|x| x.to_str().ok())
                    .and_then(|x| RsaPublicKey::from_pkcs1_pem(x).ok().into());
                Ok(Self {
                    rsa: pub_key,
                    hostname,
                    remote_addr: url,
                    conn,
                })
            } else {
                anyhow!("CRC check failed")
            }
        } else {
            anyhow!("Wrong packet type")
        }
    }
    pub async fn recv_key(mut self) -> Result<Client<Conn>> {
        let enc_msg = self.rsa.encrypt(&bincode::serialize(&Packet::new(
            self.hostname.clone(),
            self.remote_addr.clone(),
            PacketType::KeyReq,
        ))?)?;
        self.conn.write_all(&enc_msg).await?;
        let mut recv_msg = [0; RSAMSGLEN];
        self.conn.read(&mut recv_msg).await?;
        let dec_msg: Packet = bincode::deserialize(&self.rsa.decrypt(&recv_msg)?)?;
        if let PacketType::Key(k) = dec_msg.into_packet() {
            let res = Client {
                conn: Conn::new(self.conn, k.key)?,
                hostname: self.hostname,
                rsa: self.rsa,
                remote_addr: self.remote_addr,
            };
            return Ok(res);
        }
        Err(anyhow!("Got wrong packet"))
    }
}

impl Client<Conn> {
    pub async fn send(&mut self, packet: Packet) -> Result<()> {
        self.conn.send(packet).await
    }
    pub async fn recv(&mut self) -> Result<Packet> {
        self.conn.recv().await
    }
}
