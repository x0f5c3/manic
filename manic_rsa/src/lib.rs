use rsa::pkcs1::ToRsaPublicKey;
pub use rsa::{PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use zeroize::Zeroize;
pub const PADDINGFUNC: fn() -> PaddingScheme = PaddingScheme::new_oaep::<Sha256>;

#[derive(Error, Debug)]
pub enum RsaError {
    #[error("RSA error: {0}")]
    RSA(#[from] rsa::errors::Error),
    #[error("You can't encrypt to send without peer's public key")]
    NoPeerKey,
}

#[derive(Debug)]
pub struct RsaKey {
    pub public: RsaPubKey,
    private: RsaPrivKey,
    pub peer_key: Option<RsaPubKey>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RsaPubKey(RsaPublicKey);

impl AsRef<RsaPublicKey> for RsaPubKey {
    fn as_ref(&self) -> &RsaPublicKey {
        &self.0
    }
}

impl From<RsaPublicKey> for RsaPubKey {
    fn from(k: RsaPublicKey) -> Self {
        Self(k)
    }
}
impl From<&RsaPrivKey> for RsaPubKey {
    fn from(k: &RsaPrivKey) -> Self {
        RsaPublicKey::from(k.as_ref()).into()
    }
}

impl RsaPubKey {
    pub fn new(priv_key: &RsaPrivKey) -> Self {
        RsaPublicKey::from(priv_key.as_ref()).into()
    }
    pub fn encrypt(&self, data: &[u8]) -> Result<&[u8], RsaError> {
        self.0
            .encrypt(&mut rand::thread_rng(), PADDINGFUNC(), data)
            .into()
    }
}

pub struct RsaPrivKey(RsaPrivateKey);

impl AsRef<RsaPrivateKey> for RsaPrivKey {
    fn as_ref(&self) -> &RsaPrivateKey {
        &self.0
    }
}

impl From<RsaPrivateKey> for RsaPrivKey {
    fn from(k: RsaPrivateKey) -> Self {
        Self(k)
    }
}

impl Zeroize for RsaPrivKey {
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}

impl RsaPrivKey {
    pub fn new() -> Result<Self, RsaError> {
        RsaPrivateKey::new(&mut rand::thread_rng(), 2048)
            .map(|x| x.into())
            .into()
    }
    pub fn decrypt(&self, data: &[u8]) -> Result<&[u8], RsaError> {
        self.0.decrypt(PADDINGFUNC(), data).into()
    }
}

impl RsaKey {
    pub fn new_from_priv(
        priv_key: RsaPrivKey,
        peer_key: Option<RsaPubKey>,
    ) -> Result<Self, RsaError> {
        let pub_key = RsaPubKey::from(&priv_key);
        Ok(Self {
            public: pub_key,
            private: priv_key,
            peer_key: peer_key,
        })
    }
    pub fn new(peer_key: Option<RsaPubKey>) -> Result<Self, RsaError> {
        let priv_key = RsaPrivKey::new()?;
        let pub_key = RsaPubKey::from(&priv_key);
        Ok(Self {
            public: pub_key,
            private: priv_key,
            peer_key,
        })
    }
    pub fn prep_send(&self) -> Result<Vec<u8>, RsaError> {
        let pem = self.public.0.to_pkcs1_pem()?;
        self.peer_key
            .ok_or(RsaError::NoPeerKey)?
            .encrypt(&pem.into_bytes())
            .into()
    }
    pub fn encrypt(&self, data: &[u8]) -> Result<&[u8], RsaError> {
        self.peer_key.ok_or(RsaError::NoPeerKey)?.encrypt(data)
    }
    pub fn decrypt(&self, data: &[u8]) -> Result<&[u8], RsaError> {
        self.private.decrypt(data)
    }
}

impl Zeroize for RsaKey {
    fn zeroize(&mut self) {
        self.private.zeroize();
    }
}
