use crate::signature::Signature;
use crate::{codecs, CryptoError};
use aead::stream::NonceSize;
use aead::{Aead, AeadCore, AeadMut, Nonce};
use aes_gcm::Aes256Gcm;
use aes_gcm_siv::Aes256GcmSiv;
use bincode::{Decode, Encode};
use buildstructor::buildstructor;
use bytes::{Bytes, BytesMut};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use generic_array::ArrayLength;
use pin_project_lite::pin_project;
use rand_core::{OsRng, RngCore};
use rkyv::with::UnixTimestamp;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Arc;
use tokio_serde::{Deserializer, Serializer};
use zeroize::{Zeroize, ZeroizeOnDrop};

mod bincode_codec;
mod messagepack;

pub struct Crypter {
    key: Vec<u8>,
    crypt: CryptoType,
}

pub enum CryptoType {
    ChaCha20Poly1305(Arc<XChaCha20Poly1305>),
    Aes256Gcm(Arc<Aes256Gcm>),
    Aes256GcmSiv(Arc<Aes256GcmSiv>),
}

/// Marker trait for unencrypted codecs to wrap in the Messager struct
pub trait PlainText: Deserializer<Packet> + Serializer<Packet> {}

pub(crate) fn new_nonce<A: AeadCore>(rng: &mut OsRng) -> Nonce<A> {
    let mut nonce = Nonce::<A>::default();
    rng.fill_bytes(&mut nonce);
    nonce
}
pin_project! {
pub struct Messager<C, A: AeadCore> {
        #[pin]
    codec: C,
        #[pin]
    crypt: A,
    nonce: Nonce<A>,
}
    }

impl<C, A> Deserializer<Packet> for Messager<C, A>
where
    C: Deserializer<Packet> + Serializer<Packet> + DerefMut,
    <C as Deserializer<Packet>>::Error: Into<CryptoError>,
    A: Aead + AeadCore,
    CryptoError: From<<C as Deserializer<Packet>>::Error>,
{
    type Error = CryptoError;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Packet, Self::Error> {
        let this = self.project();
        let mut c = this.codec;
        let packet = c.as_mut().deserialize(src)?;
        if let MessagePayload::Encrypted(enc) = packet.data {
            let nonce = Nonce::<A>::from_slice(&enc[..24]);
            let dec = this.crypt.decrypt(nonce, &enc[24..])?;
            let res = c.deserialize(&BytesMut::from(dec.as_slice()))?;
            Ok(res)
        } else {
            Ok(packet)
        }
    }
}

impl<C, A> Serializer<Packet> for Messager<C, A>
where
    C: Deserializer<Packet> + Serializer<Packet> + DerefMut,
    A: Aead + AeadCore,
    CryptoError: From<<C as Deserializer<Packet>>::Error>,
{
    type Error = CryptoError;
    fn serialize(self: Pin<&mut Self>, item: &Packet) -> Result<Bytes, Self::Error> {
        let this = self.project();
        let mut c = this.codec;
        if let MessagePayload::Decrypted(d) = &item.data {
            let inner = c.serialize(d)?;
            let mut enc = this.crypt.encrypt(this.nonce)?;
            let mut nonce = this.nonce.to_vec();
            nonce.append(&mut enc);
            Ok(Bytes::from(nonce))
        } else {
            c.serialize(item)
        }
    }
}

pub const MAGIC_BYTES: &[u8; 5] = b"manic";
#[derive(Debug, Deserialize, Serialize, Encode, Decode, Archive)]
pub struct Packet {
    magic: [u8; 5],
    // header: u32,
    data: MessagePayload,
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
