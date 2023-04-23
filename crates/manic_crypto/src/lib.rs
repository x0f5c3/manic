mod bytes;
mod error;
mod identity;
mod signature;

use crate::typenum::U12;
use aead::Aead;
use aes_gcm::Key as AesKey;
use aes_gcm::{Aes256Gcm, Nonce};
use aes_gcm_siv::Key as SivKey;
use argon2::Argon2;
use chacha20poly1305::{Key as ChaChaKey, KeyInit, XChaCha20Poly1305, XNonce};
pub use crypto::{
    aead, cipher,
    common::{self, generic_array, rand_core, typenum},
    digest, password_hash, signature, universal_hash,
};
pub use error::CryptoError;
pub(crate) use error::Result;
use password_hash::{PasswordHasher, SaltString};
use rand_core::OsRng;
use rand_core::RngCore;

pub fn new_argon2(passphrase: &[u8]) -> Result<String> {
    let ar = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let pw = ar.hash_password(passphrase, &salt)?.to_string();
    Ok(pw)
}

pub fn encrypt_chacha(plain: &[u8], passphrase: &[u8]) -> Result<Vec<u8>> {
    let pw = new_argon2(passphrase)?;
    let key = ChaChaKey::from_slice(pw.as_bytes());
    let cipher = XChaCha20Poly1305::new(key);
    let mut nonce = XNonce::default();
    OsRng.fill_bytes(&mut nonce);
    cipher.encrypt(&nonce, plain).map_err(CryptoError::from)
}

pub fn encrypt_aes(plain: &[u8], passphrase: &[u8]) -> Result<Vec<u8>> {
    let pw = new_argon2(passphrase)?;
    let key = AesKey::<Aes256Gcm>::from_slice(pw.as_bytes());
    let cipher = Aes256Gcm::new(key);
    let mut nonce = Nonce::default();
    OsRng.fill_bytes(&mut nonce);
    cipher.encrypt(&nonce, plain).map_err(CryptoError::from)
}

pub fn encrypt_aes_siv(plain: &[u8], passphrase: &[u8]) -> Result<Vec<u8>> {}
