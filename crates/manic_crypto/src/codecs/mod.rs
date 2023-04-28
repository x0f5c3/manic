use crate::signature::Signature;
use bincode::{Decode, Encode};
use bytes::BytesMut;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand_core::{OsRng, RngCore};
use rkyv::with::UnixTimestamp;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_serde::{Deserializer, Serializer};

mod bincode_codec;
mod messagepack;

/// Marker trait for unencrypted codecs to wrap in the Messager struct
pub trait PlainText {}

pub(crate) fn new_nonce(rng: &mut OsRng) -> XNonce {
    let mut nonce = XNonce::default();
    rng.fill_bytes(&mut nonce);
    nonce
}

pub struct Messager<C: Deserializer<Packet> + Serializer<Packet>> {
    codec: C,
    crypt: XChaCha20Poly1305,
}

impl<C> Deserializer<Packet> for Messager<C>
where
    C: Deserializer<Packet> + Serializer<Packet> + Unpin,
    <C as Deserializer<Packet>>::Error: From<chacha20poly1305::Error>,
{
    type Error = <C as Deserializer<Packet>>::Error;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Packet, Self::Error> {
        let codec = Pin::new(&mut self.codec);
        let packet = codec.deserialize(src)?;
        if let MessagePayload::Encrypted(enc) = packet.data {
            let nonce = XNonce::from_slice(&enc[..24]);
            let dec = self.crypt.decrypt(nonce, &enc[24..])?;
            let res = codec.deserialize(&BytesMut::from(dec.as_slice()))?;
            Ok(res)
        } else {
            Ok(packet)
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
