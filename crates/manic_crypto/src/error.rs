use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, CryptoError>;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error(transparent)]
    IOErr(#[from] std::io::Error),
    #[error("Opaque error: {0}")]
    OpaqueKe(#[from] opaque_ke::errors::ProtocolError),
    #[error("PWHash: {0}")]
    PWHash(#[from] password_hash::Error),
    #[error(transparent)]
    Spake(#[from] spake2::Error),
    #[error("No salt")]
    NOSalt,
    #[error(transparent)]
    ChaCha(#[from] chacha20poly1305::Error),
}
