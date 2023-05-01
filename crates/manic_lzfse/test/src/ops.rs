use manic_lzfse::{LzfseRingDecoder, LzfseRingEncoder};

use std::io::{self, Read, Write};
use std::mem;

pub fn decode(decoder: &mut LzfseRingDecoder, mut src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    decoder.decode(&mut src, dst)?;
    Ok(())
}

pub fn decode_bytes(
    decoder: &mut LzfseRingDecoder,
    src: &[u8],
    dst: &mut Vec<u8>,
) -> io::Result<()> {
    decoder.decode_bytes(src, dst)?;
    Ok(())
}

pub fn decode_reader(
    decoder: &mut LzfseRingDecoder,
    src: &[u8],
    dst: &mut Vec<u8>,
) -> io::Result<()> {
    let mut rdr = decoder.reader(src);
    let mut byte = [0u8];
    while rdr.read(&mut byte)? != 0 {
        dst.push(byte[0]);
    }
    Ok(())
}

pub fn decode_reader_bytes(
    decoder: &mut LzfseRingDecoder,
    src: &[u8],
    dst: &mut Vec<u8>,
) -> io::Result<()> {
    let mut rdr = decoder.reader_bytes(src);
    let mut byte = [0u8];
    while rdr.read(&mut byte)? != 0 {
        dst.push(byte[0]);
    }
    Ok(())
}

pub fn encode(encoder: &mut LzfseRingEncoder, mut src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    encoder.encode(&mut src, dst)?;
    Ok(())
}

pub fn encode_bytes(
    encoder: &mut LzfseRingEncoder,
    src: &[u8],
    dst: &mut Vec<u8>,
) -> io::Result<()> {
    encoder.encode_bytes(src, dst)?;
    Ok(())
}

pub fn encode_writer(
    encoder: &mut LzfseRingEncoder,
    src: &[u8],
    dst: &mut Vec<u8>,
) -> io::Result<()> {
    let mut wtr = encoder.writer(dst);
    for &b in src {
        wtr.write_all(&[b])?;
    }
    wtr.finalize()?;
    Ok(())
}

pub fn encode_writer_bytes(
    encoder: &mut LzfseRingEncoder,
    src: &[u8],
    dst: &mut Vec<u8>,
) -> io::Result<()> {
    let t = mem::take(dst);
    let mut wtr = encoder.writer_bytes(t);
    for &b in src {
        wtr.write_all(&[b])?;
    }
    *dst = wtr.finalize()?;
    Ok(())
}
