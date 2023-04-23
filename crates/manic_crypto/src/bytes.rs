use crate::typenum::U12;
use bincode::enc::Encoder;
use bincode::error::EncodeError;
use bincode::{BorrowDecode, Decode, Encode};
use generic_array::typenum::U12;
use generic_array::{ArrayLength, GenericArray};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(
    Debug, Zeroize, ZeroizeOnDrop, Encode, Decode, BorrowDecode, Clone, Deserialize, Serialize,
)]
pub struct EncryptedBytes {
    payload: Vec<u8>,
    nonce: Nonce,
}

#[derive(
    Debug, Zeroize, ZeroizeOnDrop, Encode, Decode, BorrowDecode, Clone, Deserialize, Serialize,
)]
pub enum ChaChaNonce {
    Extended(chacha20poly1305::XNonce),
    Normal(chacha20poly1305::Nonce),
}

#[derive(
    Debug, Zeroize, ZeroizeOnDrop, Encode, Decode, BorrowDecode, Clone, Deserialize, Serialize,
)]
pub enum Nonce {
    ChaCha(ChaChaNonce),
    Aes(GenericArray<u8, U12>),
}

#[derive(Debug, Zeroize, ZeroizeOnDrop, Decode, BorrowDecode, Clone, Deserialize, Serialize)]
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
        }
    }
}
