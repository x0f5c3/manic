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
}

impl From<aead::Error> for CodecError {
    fn from(e: aead::Error) -> Self {
        Self::EncErr(e.to_string())
    }
}
