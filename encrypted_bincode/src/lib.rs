use bytes::{Bytes, BytesMut};
use chacha20poly1305::Key as ChaChaKey;
use chacha20poly1305::{aead::Aead, aead::NewAead, XChaCha20Poly1305, XNonce};
use log::debug;
use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io;
use std::io::ErrorKind;
use std::marker::PhantomData;
use std::pin::Pin;
use tokio_serde::{Deserializer, Serializer};
use zeroize::Zeroize;

pub type SymmetricalEncryptedBincode<T> = EncryptedBincode<T, T>;

#[derive(Clone, Debug)]
pub struct EncryptedBincode<Item, SinkItem> {
    key: Vec<u8>,
    ghost: PhantomData<(Item, SinkItem)>,
}

impl<I, S> Zeroize for EncryptedBincode<I, S> {
    fn zeroize(&mut self) {
        self.key.zeroize()
    }
}

impl<I, S> Drop for EncryptedBincode<I, S> {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl<Item, SinkItem> EncryptedBincode<Item, SinkItem> {
    pub fn new(key: Vec<u8>) -> Self {
        Self {
            key,
            ghost: PhantomData::default(),
        }
    }
}

impl<Item, SinkItem> Serializer<SinkItem> for EncryptedBincode<Item, SinkItem>
where
    SinkItem: Serialize + Debug,
{
    type Error = io::Error;

    fn serialize(self: Pin<&mut Self>, item: &SinkItem) -> Result<Bytes, Self::Error> {
        let mut nonce = XNonce::default();
        let mut rng = ChaCha20Rng::from_entropy();
        rng.fill_bytes(&mut nonce);
        let key = ChaChaKey::from_slice(self.key.as_slice());
        let cipher = XChaCha20Poly1305::new(key);
        let mut res = nonce.to_vec();
        debug!("To serialize: {:?}", item);
        let ser =
            bincode::serialize(&item).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
        debug!("Serialized: {:?}", ser);
        res.append(
            &mut cipher
                .encrypt(&nonce, ser.as_slice())
                .map_err(|e| io::Error::new(ErrorKind::InvalidData, e.to_string()))?
                .to_vec(),
        );
        debug!("Encrypted: {:?}", res);
        Ok(Bytes::from(res))
    }
}

impl<Item, SinkItem> Deserializer<Item> for EncryptedBincode<Item, SinkItem>
where
    Item: Debug,
    for<'a> Item: Deserialize<'a>,
{
    type Error = io::Error;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Item, Self::Error> {
        if src.len() < 25 {
            return Err(io::Error::new(ErrorKind::InvalidData, "Too short"));
        }
        debug!("To decrypt: {:?}", src);
        let key = ChaChaKey::from_slice(self.key.as_slice());
        let cipher = XChaCha20Poly1305::new(key);
        let dec = cipher
            .decrypt(XNonce::from_slice(&src[..24]), &src[24..])
            .map_err(|e| io::Error::new(ErrorKind::InvalidData, e.to_string()))?;
        debug!("Decrypted: {:?}", dec);
        let res: Item = bincode::deserialize(dec.as_slice())
            .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
        debug!("Deserialized: {:?}", res);
        Ok(res)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Key {
    hostname: String,
    ip: String,
    pub key: Vec<u8>,
}

impl Key {
    pub fn new(hostname: String, ip: String, key: Vec<u8>) -> Self {
        Self { hostname, ip, key }
    }
    pub fn generate(hostname: String, ip: String) -> Self {
        let mut key = chacha20poly1305::Key::default();
        let mut rng = rand::rngs::OsRng::default();
        rng.fill_bytes(&mut key);
        Self {
            hostname,
            ip,
            key: key.to_vec(),
        }
    }
}

impl Zeroize for Key {
    fn zeroize(&mut self) {
        self.hostname.zeroize();
        self.key.zeroize();
        self.ip.zeroize();
    }
}
