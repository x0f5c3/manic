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
    #[error("Wanted a 64 byte signature, got {0} bytes")]
    InvalidLen(usize),
    #[error("Decode error: {0}")]
    BincodeDecode(#[from] bincode::error::DecodeError),
    #[error("Encode error: {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),
}
