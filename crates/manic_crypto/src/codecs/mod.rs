use crate::signature::Signature;
use crate::{codecs, CryptoError};
use aead::stream::NonceSize;
use aead::{Aead, AeadCore, AeadInPlace, AeadMut, Key, KeyInit, KeySizeUser, Nonce};
use aes_gcm::aes::cipher::InvalidLength;
use aes_gcm::Aes256Gcm;
use aes_gcm_siv::Aes256GcmSiv;
use bincode::{Decode, Encode};
use buildstructor::buildstructor;
use bytes::{Bytes, BytesMut};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use generic_array::typenum::Unsigned;
use generic_array::ArrayLength;
use pin_project::pin_project;
use rand_core::{CryptoRng, OsRng, RngCore};
use rkyv::with::UnixTimestamp;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
pub use transport::{Deserializer, Serializer};
use zeroize::{Zeroize, ZeroizeOnDrop};

mod bincode_codec;
mod messagepack;
mod transport;

pub enum CryptoType {
    ChaCha20Poly1305,
    Aes256Gcm,
    Aes256GcmSiv,
}

/// Marker trait for unencrypted codecs to wrap in the Messager struct
pub trait PlainText: Deserializer + Serializer {}

pub(crate) fn new_nonce<A: AeadCore>(rng: &mut OsRng) -> Nonce<A> {
    let mut nonce = Nonce::<A>::default();
    rng.fill_bytes(&mut nonce);
    nonce
}

pub struct Messager<C, A, W>
where
    C: Deserializer<Packet> + Serializer<Packet>,
    A: Aead + AeadCore,
    W: AsyncWrite + AsyncRead,
{
    codec: C,
    crypt: A,
    writer: W,
}

#[pin_project]
pub struct Crypter<C, A: AeadCore> {
    #[pin]
    codec: C,
    #[pin]
    crypt: A,
}

impl<C, A: KeySizeUser> KeySizeUser for Crypter<C, A> {
    type KeySize = A::KeySize;

    fn key_size() -> usize {
        A::KeySize::to_usize()
    }
}

impl<C, A: KeyInit> KeyInit for Crypter<C, A> {
    fn new(key: &Key<Self>) -> Self {}

    fn new_from_slice(key: &[u8]) -> Result<Self, InvalidLength> {
        todo!()
    }

    fn generate_key(rng: impl CryptoRng + RngCore) -> Key<Self> {
        todo!()
    }
}

impl<C, A> Crypter<C, A>
where
    C: Deserializer + Serializer,
    A: Aead + AeadCore + KeyInit,
{
    pub fn new(codec: C, crypt: A) -> Self {
        Self { codec, crypt }
    }
}

impl<C, A> Deserializer for Crypter<C, A>
where
    C: Deserializer + Serializer + DerefMut,
    A: Aead + AeadCore,
    CryptoError: From<<C as Deserializer>::Error>,
{
    type Error = CryptoError;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Packet, Self::Error> {
        let this = self.project();
        let mut c = this.codec;
        let nonce = Nonce::<A>::from_slice(&src[..24]);
        let dec = this.crypt.decrypt(nonce, &src[24..])?;
        let dec = BytesMut::from(dec.as_slice());
        c.deserialize(&dec).map_err(|e| e.into())
    }
}

impl<C, A> Serializer<Packet> for Crypter<C, A>
where
    C: Serializer<Packet> + DerefMut,
    A: Aead + AeadCore + AeadInPlace,
    Result<Bytes, CryptoError>: From<Result<Bytes, <C as Serializer<Packet>>::Error>>,
    CryptoError: From<<C as Serializer<Packet>>::Error>,
{
    type Error = CryptoError;
    fn serialize(self: Pin<&mut Self>, item: &Packet) -> Result<Bytes, Self::Error> {
        let this = self.project();
        let mut c = this.codec;
        let mut enc = c.serialize(item)?;
        let nonce = new_nonce::<A>(&mut OsRng);
        let mut enc_bytes = this.crypt.encrypt(&nonce, enc.as_ref())?;
        let mut res = nonce.to_vec();
        res.append(&mut enc_bytes);
        Ok(res.into())
    }
}

pub const MAGIC_BYTES: &[u8; 5] = b"manic";
#[derive(Debug, Deserialize, Serialize, Encode, Decode, Archive)]
pub struct Packet {
    magic: [u8; 5],
    // header: u32,
    data: MessageType,
    signature: Option<Signature>,
}

/// MessagePayload is the payload either as encrypted bytes or as a MessageType
#[derive(
    Debug, Deserialize, Serialize, Encode, Decode, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub enum MessagePayload {
    Encrypted(Vec<u8>),
    Decrypted(MessageType),
}

/// MessageType is the possible payload for messaging
#[derive(
    Debug, Deserialize, Serialize, Encode, Decode, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub enum MessageType {
    PAKE { pake: Vec<u8>, curve: Vec<u8> },
    ExternalIP { external: String, bytes: Vec<u8> },
    Banner(Vec<u64>),
    Finished,
    Error(String),
    CloseRecipient,
    CloseSender,
    RecipientReady(RemoteFileRequest),
    FileInfo(SenderInfo),
}

/// SenderInfo lists the files to be transferred
#[derive(
    Debug, Serialize, Deserialize, Encode, Decode, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct SenderInfo {
    to_transfer: Vec<FileInfo>,
    empty_dirs_to_transfer: Vec<FileInfo>,
    total_no_folders: i32,
    machine_id: String,
    ask: bool,
    sending_text: bool,
    no_compress: bool,
    hashed: bool,
}

#[derive(
    Encode, Serialize, Deserialize, Decode, Debug, Archive, RkyvSerialize, RkyvDeserialize,
)]
/// FileInfo registers the information about the file
pub struct FileInfo {
    name: String,
    folder_remote: String,
    folder_source: String,
    hash: Vec<u8>,
    size: i64,
    #[with(UnixTimestamp)]
    mod_time: std::time::SystemTime,
    is_compressed: bool,
    is_encrypted: bool,
    symlink: String,
    mode: u32,
    temp_file: bool,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Archive)]
pub struct RemoteFileRequest {
    current_file_chunk_ranges: Vec<i64>,
    files_to_transfer_current_num: i32,
    machine_id: String,
}
