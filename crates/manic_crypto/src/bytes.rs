use crate::signature::Signature;
use crate::CryptoError;
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::DecodeError::ArrayLengthMismatch;
use bincode::error::{DecodeError, EncodeError};
use bincode::{BorrowDecode, Decode, Encode};
use chacha20poly1305::XNonce;
use crypto_common::typenum::U24;
use generic_array::typenum::U12;
use generic_array::{ArrayLength, GenericArray};
use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Zeroize, ZeroizeOnDrop, Encode, Decode, Clone, Deserialize, Serialize)]
pub struct EncryptedBytes {
    payload: Vec<u8>,
    nonce: Vec<u8>,
}

#[derive(Debug, Zeroize, ZeroizeOnDrop, Decode, Clone, Deserialize, Serialize)]
pub enum Bytes {
    Encrypted(EncryptedBytes),
    #[serde(skip_serializing)]
    Decrypted(Vec<u8>),
}

impl Bytes {
    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(_))
    }
    pub fn to_vec(&self) -> Option<Vec<u8>> {
        match self {
            Self::Encrypted(enc) => None,
            Self::Decrypted(dec) => Some(dec.clone()),
        }
    }
    pub fn to_bincode(&self, with_decrypted: bool) -> Result<Vec<u8>, CryptoError> {
        match self {
            Self::Encrypted(enc) => {
                bincode::encode_to_vec(&enc, bincode::config::standard()).map_err(CryptoError::from)
            }
            Self::Decrypted(dec) => {
                if with_decrypted {
                    bincode::encode_to_vec(&dec, bincode::config::standard())
                        .map_err(CryptoError::from)
                } else {
                    Err(CryptoError::Encrypted)
                }
            }
        }
    }
}

impl Encode for Bytes {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Self::Encrypted(enc) => Encode::encode(enc, encoder),
            _ => Err(EncodeError::Other("Cannot encode unencrypted bytes")),
        }
    }
}

pub const MAGIC_BYTES: &[u8; 5] = b"manic";

#[derive(Debug, Deserialize, Serialize, Encode, Decode)]
pub struct Packet {
    magic: [u8; 5],
    // header: u32,
    data: Bytes,
    signature: Option<Signature>,
}
