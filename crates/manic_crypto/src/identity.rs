use chacha20poly1305::aead::{Aead, AeadCore, AeadInPlace};

use ed25519_dalek::{SecretKey, SigningKey, VerifyingKey, Digest};
use sha2::Sha512;


use signature::Signer;
use crate::bytes::Bytes;
use crate::CryptoError;
use crate::Result;

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

pub struct Message {
    payload: Bytes,
    signature: Option<[u8; 64]>,
}

impl Message {
    pub fn sign(&mut self, key: SigningKey) -> Result<()> {
        if self.signature.is_some() {
            return Ok(());
        }
        match &self.payload {
            Bytes::Decrypted(buf) => {
                let mut h = Sha512::default();
                h.update(buf.as_slice());
                let sig = key.sign_prehashed(h, Some(SIGCTX))?.to_bytes();
                self.signature = Some(sig);
                Ok(())
            }
            _ => {
                Err(CryptoError::Encrypted)
            }
        }
    }
}
