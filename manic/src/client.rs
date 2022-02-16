use anyhow::{anyhow, Context, Result};
use futures::sink::SinkExt;
use futures::StreamExt;
use manic_proto::PacketType::Key;
use manic_proto::{
    ChaCha20Rng, EncryptedBincode, Header, RsaPublicKey, SymmetricalEncryptedBincode,
};
use manic_proto::{ChaChaKey, Packet, PacketType, PADDINGFUNC};
use manic_proto::{Key, RsaKey};
use manic_rsa::{PublicKey, RsaPubKey};
use rand_core::{RngCore, SeedableRng};
use std::io::{Read, Write};
use std::net::IpAddr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio_serde::formats::{Bincode, SymmetricalBincode};
use tokio_serde::{Framed, SymmetricallyFramed};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

const RSAMSGLEN: usize = 512;

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

pub trait Net {}

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
        let mut to_recv = [0; std::mem::size_of::<RsaPublicKey>()];
        conn.read(&mut to_recv).await?;
        let client_key: RsaPubKey = bincode::deserialize(&to_recv)?;
        let key = RsaKey::new(client_key)?;
        let to_send = &key.encrypt(&bincode::serialize(&key.public)?)?;
        conn.write_all(to_send).await?;
        let hostname = hostname::get()?.to_str().unwrap().to_string();
        Ok(Self {
            rsa: key,
            client_key,
            hostname,
            remote_addr: conn.peer_addr()?.to_string(),
            conn,
        })
    }
    pub async fn send_key(mut self) -> Result<Server<Conn>> {
        let key = Key::generate(self.hostname, self.remote_addr.clone());
        let mut to_recv = [0; RSAMSGLEN];
        self.conn.read(&mut to_recv);
        let dec = bincode::deserialize(&self.rsa.decrypt(&to_recv)?)?;
        if &dec == b"KEY" {
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
                client_key: self.client_key,
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
        let key = RsaKey::new();
        let to_send = bincode::serialize(&key.public)?;
        conn.write_all(&to_send).await?;
        let hostname = hostname::get()?.to_str().unwrap().to_string();
        let mut recv_key = [0; std::mem::size_of::<RsaPublicKey>()];
        conn.read(&mut recv_key).await?;
        let host_key: RsaPublicKey = bincode::deserialize(&key.decrypt(&recv_key))?;
        Ok(Self {
            rsa: key,
            server_key: host_key,
            hostname,
            remote_addr: url,
            conn,
        })
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
                server_key: self.server_key,
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
