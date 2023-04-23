use chacha20poly1305::aead::{Aead, AeadCore, AeadInPlace};
use chacha20poly1305::XNonce;
use ed25519_dalek::{SecretKey, SigningKey, VerifyingKey};

const SIGCTX: &[u8] = b"maniccryptoed25519sign";

pub struct CryptoContext<A>
where
    A: Aead + AeadCore + AeadInPlace,
{
    aead: A,
    key: SigningKey,
}

#[derive(Debug, Copy, Clone)]
pub struct Signature {
    inner: [u8; 64],
}

impl From<ed25519_dalek::Signature> for Signature {
    fn from(value: ed25519_dalek::Signature) -> Self {
        Self {
            inner: value.to_bytes(),
        }
    }
}

pub struct EncryptedBytes {
    payload: Vec<u8>,
    nonce: XNonce,
}

pub enum Payload {
    Decrypted(Vec<u8>),
}

pub struct Message {
    payload: Vec<u8>,
    signature: [u8; 64],
}
