use crate::base::MagicBytes;
use crate::decode::Take;
use crate::ops::{Len, ReadData, WriteData};
use crate::types::ShortBuffer;

use super::constants::*;
use super::error_kind::VnErrorKind;

#[derive(Copy, Clone, Debug)]
pub struct VnBlock {
    n_raw_bytes: u32,
    n_payload_bytes: u32,
}

impl VnBlock {
    pub fn new(n_raw_bytes: u32, n_payload_bytes: u32) -> crate::Result<Self> {
        if n_payload_bytes < 8 {
            Err(VnErrorKind::BadPayloadCount(n_payload_bytes).into())
        } else {
            Ok(Self { n_raw_bytes, n_payload_bytes })
        }
    }

    pub fn load_short<I: Copy + ShortBuffer>(&mut self, mut src: I) -> crate::Result<u32> {
        assert!(VN_HEADER_SIZE <= I::SHORT_LIMIT);
        self.load(src.take(VN_HEADER_SIZE)?.short_bytes())
    }

    #[rustfmt::skip]
    pub fn load(&mut self, src: &[u8]) -> crate::Result<u32> {
        let mut src = &src[..VN_HEADER_SIZE as usize];
        let magic_bytes      = src.read_u32();
        assert_eq!(magic_bytes, MagicBytes::Vxn.into());
        self.n_raw_bytes     = src.read_u32();
        self.n_payload_bytes = src.read_u32();
        Ok(VN_HEADER_SIZE)
    }

    pub fn store(&self, mut dst: &mut [u8]) {
        assert_eq!(dst.len(), VN_HEADER_SIZE as usize);
        dst.write_u32(MagicBytes::Vxn.into());
        dst.write_u32(self.n_raw_bytes as u32);
        dst.write_u32(self.n_payload_bytes as u32);
    }

    #[inline(always)]
    pub fn n_payload_bytes(&self) -> u32 {
        self.n_payload_bytes
    }

    #[inline(always)]
    pub fn n_raw_bytes(&self) -> u32 {
        self.n_raw_bytes
    }
}

impl Default for VnBlock {
    #[inline(always)]
    fn default() -> Self {
        Self { n_payload_bytes: 0, n_raw_bytes: 0 }
    }
}
