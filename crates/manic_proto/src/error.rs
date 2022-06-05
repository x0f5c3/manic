use chacha20poly1305::aead;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum CodecError {
    #[error("{0}")]
    IOErr(#[from] std::io::Error),
    #[error("AEAD: {0}")]
    EncErr(String),
    #[error("Bincode: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("Encrypted data too short, wanted at least 25, gotten {0}")]
    TooShort(usize),
    #[error("Compression error: {0}")]
    GZPErr(#[from] gzp::GzpError),
    #[error("Wrong magic bytes, wanted: {MAGIC_BYTES:?}, gotten: {0:?}")]
    MagicBytes([u8; 4]),
    #[error("SPAKE: {0}")]
    SPAKE(#[from] SpakeError),
    #[error("Argon2 PWHash: {0}")]
    PWHash(String),
    #[error("No salt")]
    NOSalt,
}

impl From<aead::Error> for CodecError {
    fn from(e: aead::Error) -> Self {
        Self::EncErr(e.to_string())
    }
}
