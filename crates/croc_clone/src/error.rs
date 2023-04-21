use crate::MAGIC_BYTES;
use common::argon2::password_hash;
use common::chacha20poly1305::aead;
use common::{bincode, spake2};
// use std::io::{Error, ErrorKind};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CrocError>;
#[derive(Debug, Error)]
pub enum CrocError {
    #[error("{0}")]
    IOErr(#[from] std::io::Error),
    #[error("AEAD: {0}")]
    EncErr(String),
    #[error("Bincode decode: {0}")]
    Bincode(#[from] bincode::error::DecodeError),
    #[error("Wrong magic bytes, wanted: {MAGIC_BYTES:?}, gotten: {0:?}")]
    MagicBytes([u8; 5]),
    #[error("Bincode encode: {0}")]
    BincodeEnc(#[from] bincode::error::EncodeError),
    #[error("Encrypted data too short, wanted at least 25, gotten {0}")]
    TooShort(usize),
    #[error("Argon2 PWHash: {0}")]
    PWHash(String),
    #[error("SPAKE: {0}")]
    SPAKE(#[from] SpakeError),
    #[error("No salt")]
    NOSalt,
    #[error("Bad filename")]
    BadFileName,
}

impl From<password_hash::Error> for CrocError {
    fn from(e: password_hash::Error) -> Self {
        Self::PWHash(e.to_string())
    }
}

impl From<aead::Error> for CrocError {
    fn from(e: aead::Error) -> Self {
        Self::EncErr(e.to_string())
    }
}

impl From<spake2::Error> for CrocError {
    fn from(e: spake2::Error) -> Self {
        Self::SPAKE(e.into())
    }
}

// impl Into<std::io::Error> for CodecError {
//     fn into(self) -> Error {
//         if let Self::IOErr(e) = self {
//             return e;
//         } else {
//             Error::new(ErrorKind::InvalidData, "error not due to io")
//         }
//     }
// }
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
    fn from(e: spake2::Error) -> Self {
        match e {
            spake2::Error::BadSide => Self::BadSide,
            spake2::Error::CorruptMessage => Self::CorruptMessage,
            spake2::Error::WrongLength => Self::WrongLength,
        }
    }
}

pub enum PWHashError {
    Algorithm,
    B64,
}
