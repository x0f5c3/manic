use chacha20poly1305::aead;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum CodecError {
    #[error("{0}")]
    IOErr(#[from] std::io::Error),
    #[error("AEAD: {0}")]
    EncErr(#[from] aead::Error),
    #[error("Bincode: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("Encrypted data too short, wanted at least 25, gotten {0}")]
    TooShort(usize),
}
