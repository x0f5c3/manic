use thiserror::Error;
use chacha20poly1305::aead;
use crate::MAGIC_BYTES;
use argon2::password_hash;


#[derive(Debug, Error)]
pub enum CodecError {
    #[error("{0}")]
    IOErr(#[from] std::io::Error),
    #[error("AEAD: {0}")]
    EncErr(#[from] aead::Error),
    #[error("Bincode: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("Wrong magic bytes, wanted: {MAGIC_BYTES}, gotten: {0}")]
    MagicBytes(Vec<u8>),
    #[error("SPAKE: {0}")]
    SPAKE(#[from] spake2::Error),
    #[error("Argon2 PWHash: {0}")]
    PWHash(#[from] password_hash::Error),
    #[error("No salt")]
    NOSalt,
}