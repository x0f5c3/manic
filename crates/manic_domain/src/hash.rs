//! This provides functionality for hashing a file with `BLAKE3`, using a stream reader to keep memory usage low.

use core::primitives::BLOCK_SIZE;
use std::fmt;
use std::{
    cell::RefCell,
    io::{Read, Seek},
};

use crate::hasher::Hasher;

#[derive(Debug)]
pub enum Error {
    ResetCursorPosition,
    ReadData,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ResetCursorPosition => f.write_str("Unable to reset cursor position"),
            Error::ReadData => f.write_str("Unable to read data"),
        }
    }
}

impl std::error::Error for Error {}

pub struct Request<R: Read + Seek> {
    pub reader: RefCell<R>,
}

pub fn execute<R: Read + Seek>(mut hasher: impl Hasher, req: Request<R>) -> Result<String, Error> {
    req.reader
        .borrow_mut()
        .rewind()
        .map_err(|_| Error::ResetCursorPosition)?;

    let mut buffer = vec![0u8; BLOCK_SIZE].into_boxed_slice();

    loop {
        let read_count = req
            .reader
            .borrow_mut()
            .read(&mut buffer)
            .map_err(|_| Error::ReadData)?;
        hasher.write(&buffer[..read_count]);
        if read_count != BLOCK_SIZE {
            break;
        }
    }

    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hasher::Blake3Hasher;
    use rand::RngCore;
    use std::io::Cursor;

    #[test]
    fn should_hash_string() {
        let text = "Hello world";
        let mut bytes = text.as_bytes();
        let reader = Cursor::new(&mut bytes);

        let req = Request {
            reader: RefCell::new(reader),
        };

        match execute(Blake3Hasher::default(), req) {
            Err(_) => unreachable!(),
            Ok(hash) => {
                assert_eq!(hash, blake3::hash(text.as_bytes()).to_hex().to_string());
            }
        }
    }

    #[test]
    fn should_hash_big_string() {
        #[allow(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss
        )]
        let capacity = (BLOCK_SIZE as f32 * 1.5) as usize;
        let mut buf = Vec::with_capacity(capacity);
        rand::thread_rng().fill_bytes(&mut buf);

        let orig_buf = buf.clone();
        let reader = Cursor::new(&mut buf);

        let req = Request {
            reader: RefCell::new(reader),
        };

        match execute(Blake3Hasher::default(), req) {
            Err(_) => unreachable!(),
            Ok(hash) => {
                assert_eq!(hash, blake3::hash(&orig_buf).to_hex().to_string());
            }
        }
    }

    #[test]
    fn should_reset_position_and_make_hash() {
        let text = "Hello world";
        let mut bytes = text.as_bytes();
        let mut reader = Cursor::new(&mut bytes);

        reader.seek(std::io::SeekFrom::End(0)).unwrap();

        let req = Request {
            reader: RefCell::new(reader),
        };

        match execute(Blake3Hasher::default(), req) {
            Err(_) => unreachable!(),
            Ok(hash) => {
                assert_eq!(hash, blake3::hash(text.as_bytes()).to_hex().to_string());
            }
        }
    }
}
