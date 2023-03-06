use crate::xxwriter::XXWriter;
use crate::Result;
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use std::io::prelude::*;
use tracing::debug;

pub struct CompressionData {
    data: Vec<u8>,
    compressed: bool,
    encrypted: bool,
    signed: bool,
    compression_level: Compression,
    xxhash: u64,
}

impl CompressionData {
    pub fn new_compress(data: &[u8], level: u32) -> Result<Self> {
        let xxhash_w = compress_level(data, level)?;
        let xxhash = xxhash_w.digest();
        let data = xxhash_w.into_inner();
        let compressed = true;
        let encrypted = false;
        let signed = false;
        let compression_level = Compression::new(level);
        Ok(Self {
            data,
            compressed,
            encrypted,
            signed,
            compression_level,
            xxhash,
        })
    }
}

pub fn compress_level(data: &[u8], level: u32) -> Result<XXWriter<Vec<u8>>> {
    let mut out = Vec::new();
    let mut w = DeflateEncoder::new(XXWriter::new(out), Compression::new(level));
    w.write_all(data)?;
    let crc_w = w.finish()?;
    Ok(crc_w)
}

pub fn compress(data: &[u8]) -> Result<XXWriter<Vec<u8>>> {
    compress_level(data, 2)
}

pub fn compress_io<R: Read, W: Write>(mut src: R, dst: W, level: u32) -> Result<()> {
    let mut w = DeflateEncoder::new(dst, Compression::new(level));
    match std::io::copy(&mut src, &mut w) {
        Ok(n) => {
            debug!("Written {} to writer", n);
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

pub fn decompress_io<R: Read, W: Write>(src: R, mut dst: W) -> Result<()> {
    let mut dec = DeflateDecoder::new(src);
    std::io::copy(&mut dec, &mut dst)?;
    Ok(())
}
