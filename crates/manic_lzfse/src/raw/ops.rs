use crate::lz::LzWriter;
use crate::types::{ByteReader, ShortBuffer, ShortWriter};

use super::block::{RawBlock, RAW_HEADER_SIZE};

use std::io;

pub fn raw_probe<I>(src: I) -> crate::Result<(u32, u32)>
where
    I: Copy + ShortBuffer,
{
    let mut block = RawBlock::default();
    block.load_short(src)?;
    let n_payload_bytes = RAW_HEADER_SIZE + block.n_raw_bytes();
    let n_raw_bytes = block.n_raw_bytes();
    Ok((n_payload_bytes, n_raw_bytes))
}

pub fn raw_compress<I, O>(dst: &mut O, src: I) -> io::Result<()>
where
    I: ShortBuffer,
    O: ShortWriter,
{
    assert!(src.len() <= i32::MAX as usize);
    let block = RawBlock::new(src.len() as u32);
    block.store(dst.short_block(RAW_HEADER_SIZE)?);
    dst.write_long(src)?;
    Ok(())
}

#[allow(dead_code)]
pub fn raw_decompress<I, O>(dst: &mut O, src: &mut I) -> crate::Result<()>
where
    I: for<'a> ByteReader<'a>,
    O: LzWriter,
{
    assert!(RAW_HEADER_SIZE as usize <= I::VIEW_LIMIT);
    let mut block = RawBlock::default();
    src.fill()?;
    src.skip(block.load_short(src.view())? as usize);
    block.decode(dst, src)?;
    Ok(())
}
