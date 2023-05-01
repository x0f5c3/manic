use crate::base::MagicBytes;
use crate::error::Error;
use crate::fse::FseCore;
use crate::lz::LzWriter;
use crate::raw::RawBlock;
use crate::types::ByteReader;
use crate::vn::VnCore;

use std::convert::TryInto;
use std::fmt;

/// LZFSE decoder.
///
///
/// This basic implementation decodes byte slices into byte vectors.
#[derive(Default)]
pub struct LzfseDecoder {
    pub(super) fse_core: FseCore,
    n_payload_bytes: u64,
    dst_mark: u64,
}

// Implementation notes:
//
// Higher-Rank Trait Bounds (HRTB): `for<>`
// https://stackoverflow.com/questions/35592750/how-does-for-syntax-differ-from-a-regular-lifetime-bound/35595491#35595491

impl LzfseDecoder {
    /// Decode `src` into `dst` returning the number of bytes written into `dst`.
    ///
    /// # Errors
    ///
    /// * [Error](crate::Error) detailing the nature of any errors.
    ///
    /// # Aborts
    ///
    /// With limited system memory [Vec] may abort when attempting to allocate sufficient memory.
    /// This issue will be resolved in future releases when [try_reserve()](Vec::try_reserve) is
    /// stabilized.
    ///
    /// # Examples
    ///
    /// ```
    /// use manic_lzfse::LzfseDecoder;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     // "test" string encoded.
    ///     let enc = vec![
    ///         0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74, 0x62, 0x76,
    ///         0x78, 0x24,
    ///     ];
    ///     let mut decoder = LzfseDecoder::default();
    ///     let mut dec = Vec::default();
    ///     let n_bytes = decoder.decode_bytes(&enc, &mut dec)?;
    ///     assert_eq!(n_bytes, 4);
    ///     assert_eq!(dec, b"test");
    ///     Ok(())
    /// }
    /// ```
    pub fn decode_bytes(&mut self, mut src: &[u8], dst: &mut Vec<u8>) -> crate::Result<u64> {
        let src_len = src.len();
        let dst_len = dst.len();
        self.execute(dst, &mut src).map(|u| {
            debug_assert_eq!(u.0, src_len as u64);
            debug_assert_eq!(dst_len as u64 + u.1, dst.len() as u64);
            u.1
        })
    }

    #[inline(always)]
    pub(super) fn execute<I: for<'a> ByteReader<'a>, O: LzWriter>(
        &mut self,
        dst: &mut O,
        src: &mut I,
    ) -> crate::Result<(u64, u64)> {
        self.n_payload_bytes = 0;
        self.dst_mark = dst.n_raw_bytes();
        loop {
            src.fill()?;
            if src.len() < 4 {
                return Err(Error::PayloadUnderflow);
            }
            let magic_bytes: MagicBytes = src.peek_u32().try_into()?;
            match magic_bytes {
                MagicBytes::Vx1 => self.vx1(dst, src)?,
                MagicBytes::Vx2 => self.vx2(dst, src)?,
                MagicBytes::Vxn => self.vxn(dst, src)?,
                MagicBytes::Raw => self.raw(dst, src)?,
                MagicBytes::Eos => break,
            }
        }
        if src.len() != 4 || !src.is_eof() {
            return Err(Error::PayloadOverflow);
        }
        src.skip(4);
        self.n_payload_bytes += 4;
        Ok((self.n_payload_bytes, dst.n_raw_bytes() - self.dst_mark))
    }

    #[cold]
    fn vx1<I, O>(&mut self, dst: &mut O, src: &mut I) -> crate::Result<()>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        let view = src.view();
        let n = self.fse_core.load_v1(view)?;
        src.skip(n as usize);
        self.n_payload_bytes += n as u64;
        self.vx1_vx2_cont(dst, src)
    }

    #[cold]
    fn vx2<I, O>(&mut self, dst: &mut O, src: &mut I) -> crate::Result<()>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        let view = src.view();
        let n = self.fse_core.load_v2(view)?;
        src.skip(n as usize);
        self.n_payload_bytes += n as u64;
        self.vx1_vx2_cont(dst, src)
    }

    fn vx1_vx2_cont<I, O>(&mut self, dst: &mut O, src: &mut I) -> crate::Result<()>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        let view = src.view();
        let n = self.fse_core.load_literals(view)?;
        src.skip(n as usize);
        self.n_payload_bytes += n as u64;
        let view = src.view();
        let n = self.fse_core.decode(dst, view)?;
        src.skip(n as usize);
        self.n_payload_bytes += n as u64;
        Ok(())
    }

    #[cold]
    fn vxn<I, O>(&mut self, dst: &mut O, src: &mut I) -> crate::Result<()>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        let mut core = VnCore::default();
        let view = src.view();
        let n = core.load_short(view)?;
        src.skip(n as usize);
        self.n_payload_bytes += n as u64;
        let n = core.decode(dst, src)?;
        self.n_payload_bytes += n as u64;
        Ok(())
    }

    #[cold]
    fn raw<I, O>(&mut self, dst: &mut O, src: &mut I) -> crate::Result<()>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        let mut block = RawBlock::default();
        let view = src.view();
        let n = block.load_short(view)?;
        src.skip(n as usize);
        self.n_payload_bytes += n as u64;
        let n = block.decode(dst, src)?;
        self.n_payload_bytes += n as u64;
        Ok(())
    }
}

impl fmt::Debug for LzfseDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LzfseDecoder").finish()
    }
}
