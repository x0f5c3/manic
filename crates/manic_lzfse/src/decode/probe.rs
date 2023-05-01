use crate::base::MagicBytes;
use crate::error::Error;
use crate::fse;
use crate::ops::{PeekData, Skip};
use crate::raw;
use crate::vn;

use std::convert::TryInto;

#[allow(dead_code)]
pub fn probe(mut src: &[u8]) -> crate::Result<u64> {
    let mut t_raw_bytes: u64 = 0;
    loop {
        if src.len() < 4 {
            return Err(Error::PayloadUnderflow);
        }
        let magic_bytes: MagicBytes = src.peek_u32().try_into()?;
        let (n_payload_bytes, n_raw_bytes) = match magic_bytes {
            MagicBytes::Vx1 => fse::v1_probe(src)?,
            MagicBytes::Vx2 => fse::v2_probe(src)?,
            MagicBytes::Vxn => vn::vn_probe(src)?,
            MagicBytes::Raw => raw::raw_probe(src)?,
            MagicBytes::Eos => break,
        };
        if n_payload_bytes as usize >= src.len() {
            return Err(Error::PayloadUnderflow);
        }
        src.skip(n_payload_bytes as usize);
        t_raw_bytes += n_raw_bytes as u64;
    }
    if src.len() != 4 {
        return Err(Error::PayloadOverflow);
    }
    Ok(t_raw_bytes)
}
