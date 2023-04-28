use crate::codecs::MAGIC_BYTES;
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, CryptoError>;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error(transparent)]
    IOErr(#[from] std::io::Error),
    #[error("PWHash: {0}")]
    PWHash(#[from] password_hash::Error),
    #[error(transparent)]
    Spake(#[from] spake2::Error),
    #[error("No salt")]
    NOSalt,
    #[error(transparent)]
    ChaCha(#[from] chacha20poly1305::Error),
    #[error("Wanted a {0} bytes, got {1} bytes")]
    InvalidLen(usize, usize),
    #[error("Decode error: {0}")]
    BincodeDecode(#[from] bincode::error::DecodeError),
    #[error("Encode error: {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error("Cannot sign or hash encrypted bytes")]
    Encrypted,
    #[error("Signature error: {0}")]
    SignatureError(#[from] signature::Error),
    #[error("DEFLATE compress error: {0}")]
    FlateCompressError(#[from] flate2::CompressError),
    #[error("DEFLATE decompress error: {0}")]
    FlateDecompressError(#[from] flate2::DecompressError),
    #[error("MessagePack decode error: {0}")]
    MSGPDecodeError(#[from] rmp_serde::decode::Error),
    #[error("MessagePack encode error: {0}")]
    MSGPEncodeError(#[from] rmp_serde::encode::Error),
    #[error("Wrong magic bytes, wanted: {MAGIC_BYTES:?}, gotten: {0:?}")]
    MagicBytes([u8; 5]),
    #[error("Encrypted data too short, wanted at least 25, gotten {0}")]
    TooShort(usize),
}

impl CryptoError {
    pub fn invalid_len(wanted: usize, got: usize) -> Self {
        Self::InvalidLen(wanted, got)
    }
}
