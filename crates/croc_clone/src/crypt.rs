use crate::{CrocError, Result};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use chacha20poly1305::aead::NewAead;
use chacha20poly1305::{
    aead::{Aead, AeadCore},
    Key, XChaCha20Poly1305,
};
use rand_core::{CryptoRng, OsRng, RngCore};

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
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    cipher.encrypt(&nonce, plain).map_err(CrocError::from)
}
