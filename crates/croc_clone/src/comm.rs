use crate::error::CrocError;
use crate::Message;
use bincode::config::standard;
use common::argon2::password_hash::SaltString;
use common::argon2::{Argon2, PasswordHasher};
use common::bytes::{Bytes, BytesMut};
use common::chacha20poly1305::{
    aead::{Aead, NewAead},
    Key, XChaCha20Poly1305, XNonce,
};
use common::flate2::Compression;
use common::futures::{ready, Sink, SinkExt, Stream, StreamExt, TryFutureExt, TryStream};
use common::pin_project_lite::pin_project;
use common::rand_core::{OsRng, RngCore, SeedableRng};
use common::tokio;
use common::tokio::io::Interest;
use common::tokio::time::timeout;
use common::tokio_serde::{Deserializer, Serializer};
use common::tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use common::tracing::debug;
use common::zeroize::Zeroize;
use common::{bincode, flate2, spake2};
use spake2::{Ed25519Group, Identity, Password};
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpSocket, TcpStream};

pub struct StdConn {
    st: TcpStream,
    key: Key,
    shared: String,
}

pub struct Server;

const MAGIC_BYTES: &[u8; 5] = b"manic";

impl StdConn {
    async fn connect<A: Into<SocketAddr>>(
        addr: A,
        key: Key,
        shared: String,
    ) -> Result<Self, CrocError> {
        let sock = TcpSocket::new_v4()?;
        sock.set_linger(Some(Duration::from_secs(30)))?;
        Ok(Self {
            st: sock.connect(addr.into()).await?,
            key,
            shared,
        })
    }
    async fn read(&mut self) -> Result<Vec<u8>, CrocError> {
        let mut header = [0; 5];
        self.st.read_exact(&mut header).await?;
        if &header != MAGIC_BYTES {
            return Err(CrocError::MagicBytes(header));
        }
        header = [0; 5];
        self.st.read_exact(&mut header).await?;
        let (data_size, _len): (u32, usize) =
            bincode::decode_from_slice(header.as_slice(), standard())?;
        let mut buf: Vec<u8> = (0..data_size).into_iter().map(|_| 0).collect();
        self.st.read_exact(&mut buf).await?;
        Ok(buf)
    }
    async fn write(&mut self, buf: &[u8]) -> Result<(), CrocError> {
        let header = *MAGIC_BYTES;
        self.st.write_all(&header).await?;
        let data_size = buf.len() as u32;
        self.st
            .write_all(&bincode::encode_to_vec(&data_size, standard())?)
            .await?;
        self.st
            .write_all(&bincode::encode_to_vec(buf, standard())?)
            .await?;
        Ok(())
    }

    async fn init_curve(mut self) -> Result<Comm, CrocError> {
        let (s, key) = spake2::Spake2::<Ed25519Group>::start_symmetric(
            &Password::new(&self.shared[5..]),
            &Identity::new((&self.shared[1..5]).as_ref()),
        );
        let bbytes = self.read().await?;
        let strong_key = s.finish(&bbytes)?;
        self.write(&key).await?;
        let salt = SaltString::generate(&mut OsRng);
        let pw_hash = Argon2::default().hash_password(&strong_key, &salt)?;
        self.write(pw_hash.salt.ok_or(CrocError::NOSalt)?.as_ref().as_bytes())
            .await?;
        let maybe_salt = self.read().await?;
        if maybe_salt != pw_hash.salt.ok_or(CrocError::NOSalt)?.as_ref().as_bytes() {
            return Err(CrocError::NOSalt);
        }
        Comm::new(self.st, self.key)
    }
}

pub struct Comm {
    encoded_send: Writer,
    encoded_recv: Reader,
}

impl Comm {
    pub fn new(conn: TcpStream, key: Key) -> Result<Self, CrocError> {
        let (read, write) = conn.into_split();
        let ser = Writer::new(write, Some(key.clone()));
        let de = Reader::new(read, Some(key));
        Ok(Self {
            encoded_send: ser,
            encoded_recv: de,
        })
    }
    pub async fn send(&mut self, packet: Message) -> Result<(), CrocError> {
        self.encoded_send.send(packet).await?;
        Ok(())
    }
    pub async fn recv(&mut self) -> Result<Message, CrocError> {
        let res = self
            .encoded_recv
            .next()
            .await
            .unwrap_or(Err(CrocError::NOSalt));
        res
    }
}

#[derive(Clone)]
pub struct Codec {
    key: Key,
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
    pub fn new(key: Key) -> Self {
        let cha = XChaCha20Poly1305::new(&key);
        Self { cha, key }
    }
}

impl Serializer<Message> for Codec {
    type Error = CrocError;

    fn serialize(self: Pin<&mut Self>, item: &Message) -> Result<Bytes, Self::Error> {
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

impl Deserializer<Message> for Codec {
    type Error = CrocError;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Message, Self::Error> {
        if src.len() < 25 {
            return Err(CrocError::TooShort(src.len()));
        }
        debug!("To decrypt: {:?}", src);
        let dec = self
            .cha
            .decrypt(XNonce::from_slice(&src[..24]), &src[24..])?;
        debug!("Decrypted: {:?}", dec);
        let mut decompress = flate2::read::DeflateDecoder::new(dec.as_slice());
        let res: Message =
            bincode::decode_from_std_read(&mut decompress, bincode::config::standard())?;
        debug!("Deserialized: {:?}", res);
        Ok(res)
    }
}

pub(crate) struct MultiplexedInner {
    port: u32,

}

pub struct MultiplexedConn {
    ws: Vec<Writer>,
    rs: Vec<Reader>,
}

impl MultiplexedConn {
    /// This function will block until a writer at index returned is found to be writable
    pub async fn find_available_writer(&self) -> usize {
        let res = common::futures::stream::iter(&self.ws)
            .cycle()
            .enumerate()
            .filter_map(|(i, x)| async {
                if let Ok(w) = x.writable().await {
                    if w {
                        debug!("Found available writer at index {i}");
                        Some(i)
                    }
                } else {
                    None
                }
            }).next().await
    }
}

impl Sink<Message> for MultiplexedConn {
    type Error = CrocError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {}

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
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

fn gen_key() -> Key {
    let mut key = Key::default();
    OsRng.fill_bytes(&mut key);
    key
}

impl Writer {
    pub fn new(conn: OwnedWriteHalf, key: Option<Key>) -> Self {
        let len_delim = FramedWrite::new(conn, LengthDelimitedCodec::new());

        let codec = Codec::new(key.unwrap_or_else(gen_key));
        Self {
            inner: len_delim,
            codec,
        }
    }
    pub async fn writable(&self) -> Result<bool, CrocError> {
        self.inner
            .get_ref()
            .ready(Interest::WRITABLE)
            .await
            .map(|x| x.is_writable())
            .map_err(CrocError::from)
    }
}

impl Reader {
    pub fn new(conn: OwnedReadHalf, key: Option<Key>) -> Self {
        let inner = FramedRead::new(conn, LengthDelimitedCodec::new());
        let codec = Codec::new(key.unwrap_or_else(gen_key));

        Self { inner, codec }
    }
    pub async fn readable(&self) -> Result<bool, CrocError> {
        self.inner
            .get_ref()
            .ready(Interest::READABLE)
            .await
            .map(|x| x.is_readable())
            .map_err(CrocError::from)
    }
}

impl Sink<Message> for Writer {
    type Error = CrocError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_ready(cx).map_err(CrocError::from)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        let res = self.as_mut().project().codec.serialize(&item)?;
        self.project()
            .inner
            .start_send(res)
            .map_err(CrocError::from)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_flush(cx).map_err(CrocError::from)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_close(cx).map_err(CrocError::from)
    }
}

impl Stream for Reader {
    type Item = Result<Message, CrocError>;

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

// #[cfg(test)]
// mod tests {
//     use common::tokio;
//     use super::*;
//     #[tokio::test]
//     async fn test_conn() {
//         let key = gen_key();
//         let listen_thread = tokio::spawn(async move || {
//             let listener = tokio::net::TcpListener::bind("127.0.0.1:8001").await.unwrap();
//             for (conn, raddr) in listener.accept().await {
//                 debug!("Received connection from {raddr}");
//             }
//         });
//         let conn_thread = tokio::spawn(async move || {
//             let st = tokio::net::TcpStream::connect("127.0.0.1:8001").await.unwrap();
//
//         })
//     }
// }
