use crate::error::CodecError;
use bytes::{Bytes, BytesMut};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use log::debug;
use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
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
    type Error = CodecError;

    fn serialize(self: Pin<&mut Self>, item: &SinkItem) -> std::result::Result<Bytes, Self::Error> {
        let mut nonce = XNonce::default();
        let mut rng = ChaCha20Rng::from_entropy();
        rng.fill_bytes(&mut nonce);
        let key = ChaChaKey::from_slice(self.key.as_slice());
        let cipher = XChaCha20Poly1305::new(key);
        let mut res = nonce.to_vec();
        debug!("To serialize: {:?}", item);
        let ser = bincode::serialize(&item)?;
        debug!("Serialized: {:?}", ser);
        res.append(&mut cipher.encrypt(&nonce, ser.as_slice())?.to_vec());
        debug!("Encrypted: {:?}", res);
        Ok(Bytes::from(res))
    }
}

impl<Item, SinkItem> Deserializer<Item> for EncryptedBincode<Item, SinkItem>
where
    Item: Debug,
    for<'a> Item: Deserialize<'a>,
{
    type Error = CodecError;

    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> std::result::Result<Item, Self::Error> {
        if src.len() < 25 {
            return Err(CodecError::TooShort(src.len()));
        }
        debug!("To decrypt: {:?}", src);
        let key = ChaChaKey::from_slice(self.key.as_slice());
        let cipher = XChaCha20Poly1305::new(key);
        let dec = cipher.decrypt(XNonce::from_slice(&src[..24]), &src[24..])?;
        debug!("Decrypted: {:?}", dec);
        let res: Item = bincode::deserialize(dec.as_slice())?;
        debug!("Deserialized: {:?}", res);
        Ok(res)
    }
}
