//! This module is used for standard, typical encryption and decryption.
//!
//! The data is fully loaded into memory before encryption/decryption, and it is processed within the same "block"
//!
//! # Examples
//! ```rust,ignore
//! // obviously the key should contain data, not be an empty vec
//! let raw_key = Protected::new(vec![0u8; 128]);
//! let salt = gen_salt();
//! let key = balloon_hash(raw_key, &salt, &HeaderVersion::V4).unwrap();
//! let cipher = Ciphers::initialize(key, &Algorithm::XChaCha20Poly1305).unwrap();
//!
//! let secret = "super secret information";
//!
//! let nonce = gen_nonce(&Algorithm::XChaCha20Poly1305, &Mode::MemoryMode);
//! let encrypted_data = cipher.encrypt(&nonce, secret.as_bytes()).unwrap();
//!
//! let decrypted_data = cipher.decrypt(&nonce, encrypted_data.as_slice()).unwrap();
//!
//! assert_eq!(secret, decrypted_data);
//! ```

use aead::{Aead, AeadInPlace, KeyInit, Payload};
use aes_gcm::Aes256Gcm;
use chacha20poly1305::XChaCha20Poly1305;
use deoxys::DeoxysII256;

use crate::primitives::Algorithm;
use crate::protected::Protected;

/// This `enum` defines all possible cipher types, for each AEAD that is supported by `dexios-core`
pub enum Ciphers {
    Aes256Gcm(Box<Aes256Gcm>),
    XChaCha(Box<XChaCha20Poly1305>),
    DeoxysII(Box<DeoxysII256>),
}

impl Ciphers {
    /// This can be used to quickly initialise a `Cipher`
    ///
    /// The returned `Cipher` can be used for both encryption and decryption
    ///
    /// You just need to provide the `argon2id`/`balloon` hashed key, and the algorithm to use
    ///
    /// # Examples
    /// ```rust,ignore
    /// // obviously the key should contain data, not be an empty vec
    /// let raw_key = Protected::new(vec![0u8; 128]);
    /// let salt = gen_salt();
    /// let key = balloon_hash(raw_key, &salt, &HeaderVersion::V4).unwrap();
    /// let cipher = Ciphers::initialize(key, &Algorithm::XChaCha20Poly1305).unwrap();
    /// ```
    ///
    pub fn initialize(key: Protected<[u8; 32]>, algorithm: &Algorithm) -> crate::Result<Self> {
        let cipher = match algorithm {
            Algorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new_from_slice(key.expose())?;

                Ciphers::Aes256Gcm(Box::new(cipher))
            }
            Algorithm::XChaCha20Poly1305 => {
                let cipher = XChaCha20Poly1305::new_from_slice(key.expose())?;

                Ciphers::XChaCha(Box::new(cipher))
            }
            Algorithm::DeoxysII256 => {
                let cipher = DeoxysII256::new_from_slice(key.expose())?;

                Ciphers::DeoxysII(Box::new(cipher))
            }
        };

        drop(key);
        Ok(cipher)
    }

    /// This can be used to encrypt data with a given `Ciphers` object
    ///
    /// It requires the nonce, and either some plaintext, or an `aead::Payload` (that contains the plaintext and the AAD)
    pub fn encrypt<'msg, 'aad>(
        &self,
        nonce: &[u8],
        plaintext: impl Into<Payload<'msg, 'aad>>,
    ) -> aead::Result<Vec<u8>> {
        match self {
            Ciphers::Aes256Gcm(c) => c.encrypt(nonce.as_ref().into(), plaintext),
            Ciphers::XChaCha(c) => c.encrypt(nonce.as_ref().into(), plaintext),
            Ciphers::DeoxysII(c) => c.encrypt(nonce.as_ref().into(), plaintext),
        }
    }

    pub fn encrypt_in_place(
        &self,
        nonce: &[u8],
        aad: &[u8],
        buffer: &mut dyn aead::Buffer,
    ) -> Result<(), aead::Error> {
        match self {
            Ciphers::Aes256Gcm(c) => c.encrypt_in_place(nonce.as_ref().into(), aad, buffer),
            Ciphers::XChaCha(c) => c.encrypt_in_place(nonce.as_ref().into(), aad, buffer),
            Ciphers::DeoxysII(c) => c.encrypt_in_place(nonce.as_ref().into(), aad, buffer),
        }
    }

    /// This can be used to decrypt data with a given `Ciphers` object
    ///
    /// It requires the nonce used for encryption, and either some plaintext, or an `aead::Payload` (that contains the plaintext and the AAD)
    ///
    /// NOTE: The data will not decrypt successfully if an AAD was provided for encryption, but is not present/has been modified while decrypting
    pub fn decrypt<'msg, 'aad>(
        &self,
        nonce: &[u8],
        ciphertext: impl Into<Payload<'msg, 'aad>>,
    ) -> aead::Result<Vec<u8>> {
        match self {
            Ciphers::Aes256Gcm(c) => c.decrypt(nonce.as_ref().into(), ciphertext),
            Ciphers::XChaCha(c) => c.decrypt(nonce.as_ref().into(), ciphertext),
            Ciphers::DeoxysII(c) => c.decrypt(nonce.as_ref().into(), ciphertext),
        }
    }
}
