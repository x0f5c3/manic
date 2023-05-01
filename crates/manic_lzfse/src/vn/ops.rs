use crate::lz::LzWriter;
use crate::types::{ByteReader, ShortBuffer};

use super::constants::*;
use super::{block::VnBlock, VnCore};

pub fn vn_probe<I>(src: I) -> crate::Result<(u32, u32)>
where
    I: Copy + ShortBuffer,
{
    let mut block = VnBlock::default();
    block.load_short(src)?;
    let n_payload_bytes = VN_HEADER_SIZE + block.n_raw_bytes();
    let n_raw_bytes = block.n_raw_bytes();
    Ok((n_payload_bytes, n_raw_bytes))
}

#[allow(dead_code)]
pub fn vn_decompress<I, O>(dst: &mut O, src: &mut I) -> crate::Result<()>
where
    I: for<'a> ByteReader<'a>,
    O: LzWriter,
{
    assert!(VN_HEADER_SIZE as usize <= I::VIEW_LIMIT);
    let mut core = VnCore::default();
    src.fill()?;
    src.skip(core.load_short(src.view())? as usize);
    core.decode(dst, src)?;
    Ok(())
}
