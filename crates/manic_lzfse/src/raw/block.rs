use crate::base::MagicBytes;
use crate::decode::Take;
use crate::error::Error;
use crate::lz::LzWriter;
use crate::ops::{Len, Limit, ReadData, WriteData};
use crate::types::{ByteReader, ShortBuffer};

pub const RAW_HEADER_SIZE: u32 = 0x08;

#[derive(Clone, Copy, Debug)]
pub struct RawBlock {
    n_raw_bytes: u32,
}

impl RawBlock {
    #[inline(always)]
    pub fn new(n_raw_bytes: u32) -> Self {
        Self { n_raw_bytes }
    }

    pub fn load_short<I: Copy + ShortBuffer>(&mut self, mut src: I) -> crate::Result<u32> {
        assert!(RAW_HEADER_SIZE <= I::SHORT_LIMIT);
        self.load(src.take(RAW_HEADER_SIZE)?.short_bytes())
    }

    pub fn load(&mut self, src: &[u8]) -> crate::Result<u32> {
        let mut src = &src[..RAW_HEADER_SIZE as usize];
        let magic_bytes = src.read_u32();
        assert_eq!(magic_bytes, MagicBytes::Raw.into());
        self.n_raw_bytes = src.read_u32();
        Ok(RAW_HEADER_SIZE)
    }

    pub fn store(&self, mut dst: &mut [u8]) {
        assert_eq!(dst.len(), RAW_HEADER_SIZE as usize);
        dst.write_u32(MagicBytes::Raw.into());
        dst.write_u32(self.n_raw_bytes);
    }

    #[inline(always)]
    pub fn n_raw_bytes(&self) -> u32 {
        self.n_raw_bytes
    }

    /// Decode all remaining bytes. Returning `n_payload_bytes`.
    pub fn decode<I, O>(&mut self, dst: &mut O, src: &mut I) -> crate::Result<u32>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        let n = self.n_raw_bytes as usize;
        self.copy_n(dst, src, n)?;
        self.n_raw_bytes = 0;
        Ok(n as u32)
    }

    /// Decode `n` bytes into `dst`. Returns true if `self.n_raw_bytes != 0`, that is the block
    /// is not empty.
    pub fn decode_n<I, O>(&mut self, dst: &mut O, src: &mut I, n: u32) -> crate::Result<bool>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        let n = n.min(self.n_raw_bytes);
        self.copy_n(dst, src, n as usize)?;
        self.n_raw_bytes -= n;
        Ok(self.n_raw_bytes != 0)
    }

    #[inline(always)]
    fn copy_n<I, O>(&mut self, dst: &mut O, src: &mut I, mut n: usize) -> crate::Result<()>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        loop {
            debug_assert!(n <= self.n_raw_bytes as usize);
            src.fill()?;
            let mut view = src.view();
            view.limit(n);
            dst.write_bytes_long(view)?;
            let view_len = view.len();
            n -= view_len;
            src.skip(view_len);
            if n == 0 {
                break;
            }
            if src.is_eof() {
                return Err(Error::PayloadUnderflow);
            }
        }
        Ok(())
    }
}

impl Default for RawBlock {
    #[inline(always)]
    fn default() -> Self {
        Self { n_raw_bytes: 0 }
    }
}
