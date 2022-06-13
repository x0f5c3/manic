use chacha20poly1305::aead;
use std::io::{Error, ErrorKind};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum CodecError {
    #[error("{0}")]
    IOErr(#[from] std::io::Error),
    #[error("AEAD: {0}")]
    EncErr(String),
    #[error("Bincode decode: {0}")]
    Bincode(#[from] bincode::error::DecodeError),
    #[error("Bincode encode: {0}")]
    BincodeEnc(#[from] bincode::error::EncodeError),
    #[error("Encrypted data too short, wanted at least 25, gotten {0}")]
    TooShort(usize),
    #[error("Argon2 PWHash: {0}")]
    PWHash(String),
    #[error("No salt")]
    NOSalt,
    #[error("Bad filename")]
    BadFileName,
}

impl From<aead::Error> for CodecError {
    fn from(e: aead::Error) -> Self {
        Self::EncErr(e.to_string())
    }
}

impl Into<std::io::Error> for CodecError {
    fn into(self) -> Error {
        if let Self::IOErr(e) = self {
            return e;
        } else {
            std::io::Error::new(ErrorKind::InvalidData, "error not due to io")
        }
    }
}
