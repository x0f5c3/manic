use anyhow::{anyhow, Context, Result};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use futures::SinkExt;
use manic_proto::Packet;
use manic_proto::SymmetricalCodec;
use manic_proto::{Reader, Writer};
use rand_core::OsRng;
use spake2::{Ed25519Group, Identity, Password};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_serde::formats::SymmetricalBincode;
use tokio_serde::SymmetricallyFramed;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

// pub struct Server<C: Net> {
//     rsa: RsaKey,
//     hostname: String,
//     remote_addr: String,
//     conn: C,
// }
//
// impl Server<TcpStream> {
//     pub async fn new(mut conn: TcpStream) -> Result<Self> {
//         let mut to_recv = [0; 1024];
//         let msg_len = conn.read(&mut to_recv).await?;
//         let to_recv = to_recv.into_iter().take(msg_len).collect();
//         if to_recv.len() != msg_len {
//             anyhow!("Msg not received fully")
//         }
//         let client_key: Packet = bincode::deserialize(&to_recv)?;
//         if let PacketType::RSA(msg) = client_key {
//             if msg.check_crc() {
//                 let mut key = RsaKey::new(None)?;
//                 key.peer_key = key
//                     .decrypt(&msg.data)
//                     .ok()
//                     .and_then(|x| x.to_str().ok())
//                     .and_then(|x| RsaPublicKey::from_pkcs1_pem(x).ok().into());
//                 let hostname = hostname::get()?.to_str().unwrap().to_string();
//                 let to_send = Packet::new(
//                     hostname.clone(),
//                     conn.peer_addr()?.to_string(),
//                     PacketType::new_rsa(key.prep_send()?)?.clone(),
//                 );
//                 let to_send = bincode::serialize(&to_send)?;
//                 conn.write_all(&to_send).await?;
//                 Ok(Self {
//                     rsa: key,
//                     hostname,
//                     remote_addr: conn.peer_addr()?.to_string(),
//                     conn,
//                 })
//             } else {
//                 anyhow!("CRC check failed")
//             }
//         } else {
//             anyhow!("Wrong packet type")
//         }
//     }
//     pub async fn send_key(mut self) -> Result<Server<Conn>> {
//         let key = Key::generate(self.hostname, self.remote_addr.clone());
//         let mut to_recv = [0; RSAMSGLEN];
//         self.conn.read(&mut to_recv);
//         let dec: Packet = bincode::deserialize(&self.rsa.decrypt(&to_recv)?)?;
//         if dec.into_packet() == PacketType::KeyReq {
//             let to_send = Packet::new(
//                 self.hostname.clone(),
//                 self.remote_addr.clone(),
//                 PacketType::Key(key),
//             );
//             let enc = self.rsa.encrypt(&bincode::serialize(&to_send)?)?;
//             self.conn.write(&enc).await?;
//             let conn = Conn::new(self.conn, key.key.clone())?;
//             Ok(Server {
//                 hostname,
//                 rsa: self.rsa,
//                 remote_addr: self.remote_addr,
//                 conn,
//             })
//         } else {
//             Err(anyhow!("Expected msg KEY"))
//         }
//     }
// }
//
// pub struct Client<C: Net> {
//     rsa: RsaKey,
//     hostname: String,
//     remote_addr: String,
//     conn: C,
// }
//
// impl Client<TcpStream> {
//     pub async fn new(url: String) -> Result<Self> {
//         let mut conn = TcpStream::connect(&url).await?;
//         let priv_key = RsaPrivKey::new()?;
//         let mut pub_key = RsaKey::new_from_priv(priv_key, None)?;
//         let hostname = hostname::get()?.to_str().unwrap().to_string();
//         let to_send = bincode::serialize(&Packet::new(
//             hostname.clone(),
//             url.clone(),
//             PacketType::new_rsa(pub_key.prep_send()?),
//         ))?;
//         conn.write_all(&to_send).await?;
//         let mut recv_key = [0; 2048];
//         let msg_len = conn.read(&mut recv_key).await?;
//         let recv_key = recv_key.into_iter().take(msg_len).collect();
//         if recv_key.len() != msg_len {
//             anyhow!("Message not received fully")
//         }
//         let host_key: Packet = bincode::deserialize(&key.decrypt(&recv_key)?)?;
//         if let PacketType::RSA(key_msg) = host_key {
//             if key_msg.check_crc() {
//                 pub_key.peer_key = pub_key
//                     .decrypt(&key_msg.data)
//                     .ok()
//                     .and_then(|x| x.to_str().ok())
//                     .and_then(|x| RsaPublicKey::from_pkcs1_pem(x).ok().into());
//                 Ok(Self {
//                     rsa: pub_key,
//                     hostname,
//                     remote_addr: url,
//                     conn,
//                 })
//             } else {
//                 anyhow!("CRC check failed")
//             }
//         } else {
//             anyhow!("Wrong packet type")
//         }
//     }
//     pub async fn recv_key(mut self) -> Result<Client<Conn>> {
//         let enc_msg = self.rsa.encrypt(&bincode::serialize(&Packet::new(
//             self.hostname.clone(),
//             self.remote_addr.clone(),
//             PacketType::KeyReq,
//         ))?)?;
//         self.conn.write_all(&enc_msg).await?;
//         let mut recv_msg = [0; RSAMSGLEN];
//         self.conn.read(&mut recv_msg).await?;
//         let dec_msg: Packet = bincode::deserialize(&self.rsa.decrypt(&recv_msg)?)?;
//         if let PacketType::Key(k) = dec_msg.into_packet() {
//             let res = Client {
//                 conn: Conn::new(self.conn, k.key)?,
//                 hostname: self.hostname,
//                 rsa: self.rsa,
//                 remote_addr: self.remote_addr,
//             };
//             return Ok(res);
//         }
//         Err(anyhow!("Got wrong packet"))
//     }
// }

// impl Client<Conn> {
//     pub async fn send(&mut self, packet: Packet) -> Result<()> {
//         self.conn.send(packet).await
//     }
//     pub async fn recv(&mut self) -> Result<Packet> {
//         self.conn.recv().await
//     }
// }
