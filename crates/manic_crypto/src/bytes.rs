use std::ops::Deref;
use crate::typenum::U12;
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
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Zeroize, Serialize, Deserialize)]
pub struct SizedBytesArray<N: ArrayLength<u8> + Drop> {
    arr: GenericArray<u8, N>,
}

impl<N: ArrayLength<u8> + Drop> Deref for SizedBytesArray<>

#[derive(Debug, Zeroize, ZeroizeOnDrop, Encode, Decode, Clone, Deserialize, Serialize)]
pub struct BytesArray {
    data: Vec<u8>,
}

impl<N: ArrayLength<u8>> From<GenericArray<u8, N>> for BytesArray {
    fn from(value: GenericArray<u8, N>) -> Self {
        let data = value.to_vec();
        Self { data }
    }
}

impl AsRef<[u8]> for BytesArray {
    fn as_ref(&self) -> &[u8] {
        self.data.as_slice()
    }
}

impl<N: ArrayLength<u8>> TryInto<GenericArray<u8, N>> for BytesArray {
    type Error = CryptoError;
    fn try_into(self) -> Result<GenericArray<u8, N>, Self::Error> {
        if self.data.len() != N::USIZE {
            return Err(CryptoError::InvalidLen(N::USIZE, self.data.len()));
        }
        GenericArray::from_exact_iter(self.data.into_iter())
            .ok_or(CryptoError::InvalidLen(N::USIZE, self.data.len()))
    }
}

pub enum EncryptedBuf {
    ChaChaX(GenericArray<u8, U24>),
    ChaCha(GenericArray<u8, U12>),
}

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

impl Encode for Bytes {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Self::Encrypted(enc) => {
                bincode::Encode::encode(&enc.nonce, encoder)?;
                bincode::Encode::encode(&enc.payload, encoder)
            }
            _ => Err(EncodeError::Other("Cannot encode unencrypted bytes")),
        }
    }
}
