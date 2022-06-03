use crate::MAGIC_BYTES;
use argon2::password_hash;
use chacha20poly1305::aead;
use spake2::Error;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CodecError>;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("{0}")]
    IOErr(#[from] std::io::Error),
    #[error("AEAD: {0}")]
    EncErr(#[from] aead::Error),
    #[error("Bincode: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("Wrong magic bytes, wanted: {MAGIC_BYTES:?}, gotten: {0:?}")]
    MagicBytes([u8; 4]),
    #[error("SPAKE: {0}")]
    SPAKE(#[from] SpakeError),
    #[error("Argon2 PWHash: {0}")]
    PWHash(String),
    #[error("No salt")]
    NOSalt,
    #[error("Encrypted data too short, wanted at least 25, gotten {0}")]
    TooShort(usize),
}

impl From<password_hash::Error> for CodecError {
    fn from(e: password_hash::Error) -> Self {
        Self::PWHash(e.to_string())
    }
}

impl From<spake2::Error> for CodecError {
    fn from(e: Error) -> Self {
        Self::SPAKE(e.into())
    }
}

#[derive(Debug, Error)]
pub enum SpakeError {
    #[error("Bad side")]
    BadSide,
    #[error("Corrupt message")]
    CorruptMessage,
    #[error("Invalid Length")]
    WrongLength,
}

impl From<spake2::Error> for SpakeError {
    fn from(e: Error) -> Self {
        match e {
            Error::BadSide => Self::BadSide,
            Error::CorruptMessage => Self::CorruptMessage,
            Error::WrongLength => Self::WrongLength,
        }
    }
}

pub enum PWHashError {
    Algorithm,
    B64,
}
