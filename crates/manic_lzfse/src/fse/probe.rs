use crate::types::ShortBuffer;

use super::block::FseBlock;

pub fn v1_probe<I>(src: I) -> crate::Result<(u32, u32)>
where
    I: Copy + ShortBuffer,
{
    let mut block = FseBlock::default();
    let (n_header_payload_bytes, n_weight_payload_bytes) = block.load_v1_short(src)?;
    let n_payload_bytes = n_header_payload_bytes + n_weight_payload_bytes + block.n_payload_bytes();
    let n_raw_bytes = block.n_raw_bytes();
    Ok((n_payload_bytes, n_raw_bytes))
}

pub fn v2_probe<I>(src: I) -> crate::Result<(u32, u32)>
where
    I: Copy + ShortBuffer,
{
    let mut block = FseBlock::default();
    let (n_header_payload_bytes, n_weight_payload_bytes) = block.load_v2_short(src)?;
    let n_payload_bytes = n_header_payload_bytes + n_weight_payload_bytes + block.n_payload_bytes();
    let n_raw_bytes = block.n_raw_bytes();
    Ok((n_payload_bytes, n_raw_bytes))
}
