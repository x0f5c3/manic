use crate::codecs::MAGIC_BYTES;

use color_eyre::eyre::{eyre, WrapErr};
use color_eyre::Report;
use sha2::digest::typenum::op;
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

pub(crate) type Result<T> = color_eyre::Result<T>;

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
    #[error("Cannot construct key from this slice: {0}")]
    InvalidLengthKey(#[from] crypto_common::InvalidLength),
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

#[derive(Debug)]
pub struct MultipleErrors {
    errors: Vec<Report>,
}

impl Display for MultipleErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Encountered {} errors:", self.errors.len())?;
        let last_index = self.errors.len() - 1;
        for (i, e) in &self.errors.iter().enumerate() {
            if i != last_index {
                writeln!(f, "{}", e)?;
            } else {
                write!(f, "{}", e)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for MultipleErrors {}

impl From<Vec<color_eyre::Report>> for MultipleErrors {
    fn from(errors: Vec<color_eyre::Report>) -> Self {
        Self { errors }
    }
}

impl Into<color_eyre::Report> for MultipleErrors {
    fn into(self) -> color_eyre::Report {
        eyre!(self.to_string())
    }
}

impl<TT, E> FromIterator<std::result::Result<TT, E>> for MultipleErrors
where
    E: Into<CryptoError> + Send + Sync + 'static,
{
    fn from_iter<T: IntoIterator<Item = std::result::Result<TT, E>>>(iter: T) -> Self {
        let errors = iter
            .into_iter()
            .filter(|e| e.is_err())
            .map(|e| CryptoError::from(Result::unwrap_err(e)).into())
            .collect::<Vec<color_eyre::Report>>();
        Self { errors }
    }
}

impl<TT, E> FromIterator<std::result::Result<TT, E>> for MultipleErrors
where
    E: Into<color_eyre::Report> + Send + Sync + 'static,
{
    fn from_iter<T: IntoIterator<Item = std::result::Result<TT, E>>>(iter: T) -> Self {
        let errors = iter
            .into_iter()
            .filter(|e| e.is_err())
            .map(|e| Result::unwrap_err(e).into())
            .collect();
        Self { errors }
    }
}

impl<I, T, E> From<I> for MultipleErrors
where
    I: IntoIterator<Item = std::result::Result<T, E>>,
    E: Into<CryptoError>,
{
    fn from(iter: I) -> Self {
        let mut errors: Vec<color_eyre::Report> = Vec::new();
        for res in iter {
            if let Err(e) = res {
                errors.push(CryptoError::from(e).into());
            }
        }
        Self { errors }
    }
}

impl<I, T, E> From<I> for MultipleErrors
where
    I: IntoIterator<Item = std::result::Result<T, E>>,
    E: Into<color_eyre::Report>,
{
    fn from(value: I) -> Self {
        let mut errors: Vec<color_eyre::Report> = Vec::new();
        for res in value {
            if let Err(e) = res {
                errors.push(e.into());
            }
        }
        Self { errors }
    }
}

impl Into<color_eyre::Report> for CryptoError {
    fn into(self) -> color_eyre::Report {
        match self {
            Self::Encrypted => eyre!("Cannot sign or hash encrypted bytes"),
            Self::InvalidLengthKey(e) => eyre!("Cannot construct a key from this slice: {}", e),
            Self::InvalidLen(wanted, got) => {
                eyre!("Wanted a {} bytes, got {} bytes", wanted, got)
            }
            Self::MagicBytes(bytes) => eyre!(
                "Wrong magic bytes, wanted: {:?}, gotten: {:?}",
                MAGIC_BYTES,
                bytes
            ),
            Self::MSGPDecodeError(e) => eyre!("MessagePack decode error: {}", e),
            Self::MSGPEncodeError(e) => eyre!("MessagePack encode error: {}", e),
            Self::NOSalt => eyre!("No salt"),
            Self::PWHash(e) => eyre!("PWHash: {}", e),
            Self::SignatureError(e) => eyre!("Signature error: {}", e),
            Self::TooShort(len) => eyre!(
                "Encrypted data too short, wanted at least 25, gotten {}",
                len
            ),
            Self::IOErr(e) => eyre!("IO error: {}", e),
            Self::Spake(e) => eyre!("Spake error: {}", e),
            Self::ChaCha(e) => eyre!("ChaCha error: {}", e),
            Self::BincodeDecode(e) => eyre!("Bincode decode error: {}", e),
            Self::BincodeEncode(e) => eyre!("Bincode encode error: {}", e),
            Self::FlateCompressError(e) => eyre!("DEFLATE compress error: {}", e),
            Self::FlateDecompressError(e) => eyre!("DEFLATE decompress error: {}", e),
        }
    }
}

impl CryptoError {
    pub fn invalid_len(wanted: usize, got: usize) -> Self {
        Self::InvalidLen(wanted, got)
    }
}

#[derive(Debug)]
pub struct ContextError {
    pub(crate) msg: Option<String>,
    pub(crate) err: CryptoError,
}

impl Display for ContextError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "{}: {}", msg, self.err)
        } else {
            write!(f, "{}", self.err)
        }
    }
}

impl std::error::Error for ContextError {}

impl From<CryptoError> for ContextError {
    fn from(err: CryptoError) -> Self {
        Self { msg: None, err }
    }
}

pub trait Context<T> {
    fn context(self, msg: impl Into<String>) -> std::result::Result<T, ContextError>;
    fn into_context(self) -> std::result::Result<T, ContextError>;
}

impl<T, E: Into<CryptoError>> Context<T> for std::result::Result<T, E> {
    fn context(self, msg: impl Into<String>) -> std::result::Result<T, ContextError> {
        self.map_err(|err| ContextError {
            msg: Some(msg.into()),
            err: err.into(),
        })
    }
    fn into_context(self) -> std::result::Result<T, ContextError> {
        self.map_err(|err| ContextError {
            msg: None,
            err: err.into(),
        })
    }
}

macro_rules! anyhow {
    ($e: expr) => {};
}
