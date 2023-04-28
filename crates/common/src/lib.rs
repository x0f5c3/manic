mod array;

pub use argon2::password_hash;
pub use buildstructor;
pub use password_hash::rand_core;
pub use thiserror;
pub use tokio;
pub use tokio_serde;
pub use {
    argon2, bincode, bytes, chacha20poly1305, crc, flate2, futures, pin_project_lite, rand,
    rmp_serde, serde, spake2, time, tokio_util, tracing, xxhash_rust, zeroize,
};

pub use postcard;
use postcard::de_flavors;
use serde::de::Error;
use std::io::{Read, Write};
use std::sync::Mutex;
use tokio::io::AsyncReadExt;

pub struct FlateFlavor<R, W>
where
    R: Read,
    W: Write,
{
    pub encoder: Mutex<flate2::write::DeflateEncoder<W>>,
    pub decoder: Mutex<flate2::read::DeflateDecoder<R>>,
}

impl<'de, R: Read + 'de, W: Write + 'de> de_flavors::Flavor<'de> for FlateFlavor<R, W> {
    type Remainder = Vec<u8>;
    type Source = R;

    fn pop(&mut self) -> postcard::Result<u8> {
        let mut res = [0u8; 1];
        self.decoder
            .lock()
            .map_err(|e| postcard::Error::SerdeDeCustom)?
            .read_exact(&mut res)
            .map_err(|e| postcard::Error::custom(e.to_string()))?;
        Ok(res[0])
    }

    fn try_take_n(&mut self, ct: usize) -> postcard::Result<&'de [u8]> {
        let mut res = vec![0u8; ct];
        self.decoder
            .lock()
            .map_err(|e| postcard::Error::SerdeDeCustom)?
            .read_exact(&mut res)
            .map_err(|e| postcard::Error::custom(e.to_string()))?;
        Ok(res.as_slice())
    }

    fn finalize(self) -> postcard::Result<Self::Remainder> {
        let mut res = Vec::new();
        self.decoder
            .lock()
            .map_err(|e| postcard::Error::SerdeDeCustom)?
            .read_to_end(&mut res)
            .map_err(|e| postcard::Error::custom(e.to_string()))?;
        Ok(res)
    }
}
