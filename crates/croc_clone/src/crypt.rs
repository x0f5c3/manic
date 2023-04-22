use crate::{CrocError, Result};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use chacha20poly1305::{aead::Aead, Key, XChaCha20Poly1305};
use common::chacha20poly1305::{KeyInit, XNonce};
use common::rand_core::{CryptoRng, OsRng, RngCore};
use common::{argon2, chacha20poly1305};

pub fn new_argon2(passphrase: &[u8]) -> Result<String> {
    let ar = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let pw = ar.hash_password(passphrase, &salt)?.to_string();
    Ok(pw)
}

pub fn encrypt_chacha(plain: &[u8], passphrase: &[u8]) -> Result<Vec<u8>> {
    let pw = new_argon2(passphrase)?;
    let key = Key::from_slice(pw.as_bytes());
    let cipher = XChaCha20Poly1305::new(key);
    let mut nonce = XNonce::default();
    OsRng.fill_bytes(&mut nonce);
    cipher.encrypt(&nonce, plain).map_err(CrocError::from)
}
