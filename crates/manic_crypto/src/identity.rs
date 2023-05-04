use chacha20poly1305::aead::Aead;
use std::collections::HashMap;

use ed25519_dalek::{Digest, SecretKey, SigningKey, VerifyingKey};
use x25519_dalek::EphemeralSecret;

use crate::signature::Signature;

use sha2::Sha512;

use crate::bytes::Bytes;
use crate::CryptoError;
use crate::Result;

use zeroize::{Zeroize, ZeroizeOnDrop};
use crate::error::ContextError;

pub type EDSignature = [u8; 64];

const SIGCTX: &[u8] = b"maniccryptoed25519sign";

#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct CryptoContext<A>
where
    A: Aead + Zeroize + ZeroizeOnDrop,
{
    aead: A,
    key: SecretKey,
    #[zeroize(skip)]
    clients: HashMap<String, VerifyingKey>,
}

pub struct Message {
    payload: Bytes,
    signature: Option<Signature>,
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
                let sig = key.sign_prehashed(h, Some(SIGCTX))?;
                self.signature = Some(Signature::from(sig.to_bytes()));
                Ok(())
            }
            _ => Err(ContextError::from(CryptoError::Encrypted)),
        }
    }
}
