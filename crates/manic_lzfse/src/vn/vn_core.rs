use crate::kit::W08;
use crate::lmd::{DMax, LMax, LiteralLen, MMax, MatchDistanceUnpack, MatchLen};
use crate::lz::LzWriter;
use crate::ops::{Len, Limit, ShortLimit};
use crate::types::{ByteReader, ShortBuffer, ShortBytes};

use super::block::VnBlock;
use super::constants::*;
use super::error_kind::VnErrorKind;
use super::object::Vn;
use super::opc;

use std::mem;
use std::num::NonZeroU32;

// Implementation notes:
//
// The maximum LZVN compression ratio approaches 0x0110 / 0x02 for both `LrgL` and  `LrgM` types,
// that is 0x88.
//
// The LZFSE reference, by default, will not emit LZVN blocks much larger than 0x1000 although
// it can potentially decompress them. We'll cover LZVN blocks to their theoretical limits, although
// it's unlikely well ever encounter them outside of custom/ malicious payloads.
//
// As we are using `ShortBuffer` types, unusually we may not be able to fit in an entire LZVN block.
// However as op decodes are atomic, we can intercept `Error::PayloadOverflow` errors, refill our
// buffer and continue. We'll refer to this as the `VN_PAYLOAD_LIMIT` overflow mechanism.
//
// Opcodes, by their very construction, limit literal len, match len and match distance values. The
// exact limits vary by opcode. With care, this allows us skip certain limit/ boundary checks.
const MIN_LIMIT: u32 = 0x0200;

pub struct VnCore {
    n_raw_bytes: u32,
    n_payload_bytes: u32,
    match_distance: MatchDistanceUnpack<Vn>,
}

impl VnCore {
    pub fn load_short<I: Copy + ShortBuffer>(&mut self, src: I) -> crate::Result<u32> {
        let mut block = VnBlock::default();
        let n_payload_bytes_len = block.load_short(src)?;
        self.n_raw_bytes = block.n_raw_bytes();
        self.n_payload_bytes = block.n_payload_bytes();
        self.match_distance = MatchDistanceUnpack::default();
        Ok(n_payload_bytes_len)
    }

    /// Decode all remaining bytes. Returning `n_payload_bytes`.
    pub fn decode<I, O>(&mut self, dst: &mut O, src: &mut I) -> crate::Result<u32>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        // u64::MAX used, see `decode_short`
        let n = self.n_payload_bytes;
        let ok = self.decode_mark(dst, src, u64::MAX)?;
        // Unless we hit the 16 exabyte limit, at which point we are in undefined territory, ok is
        // going to be false.
        debug_assert!(!ok);
        Ok(n)
    }

    /// Attempt to decode `n` bytes into `dst`. Returns true if `self.n_raw_bytes != 0`, that is the
    /// block is not empty.
    pub fn decode_n<I, O>(&mut self, dst: &mut O, src: &mut I, n: u32) -> crate::Result<bool>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        self.decode_mark(dst, src, dst.n_raw_bytes() + n as u64)
    }

    #[allow(clippy::assertions_on_constants)]
    #[inline(always)]
    fn decode_mark<I, O>(&mut self, dst: &mut O, src: &mut I, dst_mark: u64) -> crate::Result<bool>
    where
        I: for<'a> ByteReader<'a>,
        O: LzWriter,
    {
        assert!(MIN_LIMIT <= I::View::SHORT_LIMIT);
        assert!(MIN_LIMIT as usize <= I::VIEW_LIMIT);
        assert!(MIN_LIMIT <= VN_PAYLOAD_LIMIT);
        loop {
            src.fill()?;
            let mut view = src.view();
            view.limit(VN_PAYLOAD_LIMIT as usize);
            let view_len = view.len();
            let dst_n_raw_bytes = dst.n_raw_bytes();
            let res = self.decode_short(dst, &mut view, dst_mark);
            let n_payload_bytes_len = view_len - view.len();
            debug_assert!(n_payload_bytes_len <= VN_PAYLOAD_LIMIT as usize);
            let n_raw_bytes_len = dst.n_raw_bytes() - dst_n_raw_bytes;
            debug_assert!(n_raw_bytes_len <= u32::MAX as u64);
            if n_payload_bytes_len as u32 > self.n_payload_bytes {
                return Err(crate::Error::PayloadUnderflow);
            }
            if n_raw_bytes_len as u32 > self.n_raw_bytes {
                return Err(VnErrorKind::BadPayload.into());
            }
            self.n_payload_bytes -= n_payload_bytes_len as u32;
            self.n_raw_bytes -= n_raw_bytes_len as u32;
            let cycle = src.len() > VN_PAYLOAD_LIMIT as usize;
            debug_assert!(cycle || src.is_eof());
            src.skip(n_payload_bytes_len);
            return match res {
                Ok(true) => Ok(self.n_raw_bytes != 0),
                Ok(false) if self.n_payload_bytes != 0 => Err(crate::Error::PayloadOverflow),
                Ok(false) if self.n_raw_bytes != 0 => Err(VnErrorKind::BadPayload.into()),
                Ok(false) => Ok(false),
                Err(crate::Error::PayloadUnderflow) if cycle => continue,
                Err(err) => Err(err),
            };
        }
    }

    #[inline(always)]
    fn decode_short<I: ShortBuffer, O: LzWriter>(
        &mut self,
        dst: &mut O,
        src: &mut I,
        dst_mark: u64,
    ) -> crate::Result<bool> {
        assert!(MIN_LIMIT <= I::SHORT_LIMIT);
        if src.len() < 8 {
            return Err(crate::Error::PayloadUnderflow);
        }
        // If `dst_mark == u64;:MAX` and inlined the compiler should elide this check.
        while dst.n_raw_bytes() <= dst_mark {
            if let Some(n_payload_bytes) = unsafe { self.atomic_op(dst, src.short_bytes())? } {
                unsafe { src.skip_unchecked(n_payload_bytes.get() as usize) };
            } else {
                debug_assert!(src.len() >= 8);
                unsafe { src.skip_unchecked(8) };
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Barring write errors which will leave `dst` in an undefined state, this operation is atomic.
    ///
    /// Throws `PayloadUnderflow` error if insufficient `src` bytes are available, or if after
    /// decoding, less than 8 bytes are present in `src` unless `Op::Eof`. This latter mechanic
    /// allows us to omit a bounds check when cycling this method.
    ///
    /// Returns the number of `src` bytes consumed, or none if `Op::Eof` which implies 8 bytes
    /// consumed.
    ///
    /// # Safety
    ///
    /// * `src.len() >= 8`
    #[inline(always)]
    unsafe fn atomic_op<O>(&mut self, dst: &mut O, src: &[u8]) -> crate::Result<Option<NonZeroU32>>
    where
        O: LzWriter,
    {
        debug_assert!(src.len() >= 8);
        let opu = src.as_ptr().cast::<u32>().read_unaligned().to_le();
        let n_payload_bytes = match OP_TABLE[opu as usize & 0xFF] {
            Op::SmlL => self.typ_l(dst, src.get(1..), opc::decode_sml_l(opu))? + 1,
            Op::LrgL => self.typ_l(dst, src.get(2..), opc::decode_lrg_l(opu))? + 2,
            Op::SmlM => self.typ_m(dst, src.get(1..), opc::decode_sml_m(opu))? + 1,
            Op::LrgM => self.typ_m(dst, src.get(2..), opc::decode_lrg_m(opu))? + 2,
            Op::PreD => self.pre_d(dst, src.get(1..), opc::decode_pre_d(opu))? + 1,
            Op::SmlD => self.typ_d(dst, src.get(2..), opc::decode_sml_d(opu))? + 2,
            Op::MedD => self.typ_d(dst, src.get(3..), opc::decode_med_d(opu))? + 3,
            Op::LrgD => self.typ_d(dst, src.get(3..), opc::decode_lrg_d(opu))? + 3,
            Op::Nop => self.nop(src.get(1..))? + 1,
            Op::Eos => return self.eos(src),
            Op::Udef => return Err(VnErrorKind::BadOpcode.into()),
        };
        debug_assert!(n_payload_bytes as usize + 8 <= src.len());
        debug_assert_ne!(n_payload_bytes, 0);
        Ok(Some(NonZeroU32::new(n_payload_bytes)))
    }

    fn eos(&self, src: &[u8]) -> crate::Result<Option<NonZeroU32>> {
        if src.len() < 8 {
            return Err(crate::Error::PayloadUnderflow);
        }
        if src[..8] != [EOS, 0, 0, 0, 0, 0, 0, 0] {
            return Err(VnErrorKind::BadPayload.into());
        }
        Ok(None)
    }

    fn nop(&self, bytes: &[u8]) -> crate::Result<u32> {
        if bytes.len() < 8 {
            return Err(crate::Error::PayloadUnderflow);
        }
        Ok(0)
    }

    /// # Safety
    ///
    /// * `literal.len() <= Vn::MAX_LITERAL_LEN`
    unsafe fn typ_l<O>(&mut self, dst: &mut O, src: &[u8], literal_len: u32) -> crate::Result<u32>
    where
        O: LzWriter,
    {
        debug_assert!(literal_len <= Vn::MAX_LITERAL_LEN as u32);
        if src.len() < literal_len as usize + 8 {
            return Err(crate::Error::PayloadUnderflow);
        }
        let bytes =
            ShortBytes::<LiteralLen<Vn>, W08>::from_bytes_unchecked(src, literal_len as usize);
        dst.write_bytes_short(bytes)?;
        Ok(literal_len)
    }

    /// # Safety
    ///
    /// * `match_len != 0`
    unsafe fn typ_m<O>(&mut self, dst: &mut O, src: &[u8], match_len: u32) -> crate::Result<u32>
    where
        O: LzWriter,
    {
        if src.len() < 8 {
            return Err(crate::Error::PayloadUnderflow);
        }
        let match_len = MatchLen::new(match_len);
        write_match(dst, match_len, self.match_distance)?;
        Ok(0)
    }

    /// # Safety
    ///
    /// * `literal.len() <= size_of::<u32>()`
    /// * `match_len <= Vn::MAX_MATCH_LEN`
    /// * `match_len != 0`
    unsafe fn pre_d<O>(
        &mut self,
        dst: &mut O,
        src: &[u8],
        (literal_len, match_len): (u32, u32),
    ) -> crate::Result<u32>
    where
        O: LzWriter,
    {
        debug_assert!(literal_len <= mem::size_of::<u32>() as u32);
        debug_assert!(match_len <= Vn::MAX_MATCH_LEN as u32);
        debug_assert!(match_len != 0);
        if src.len() < literal_len as usize + 8 {
            return Err(crate::Error::PayloadUnderflow);
        }
        let bytes = src.as_ptr().cast::<u32>().read_unaligned();
        let literal_len = LiteralLen::new(literal_len);
        let match_len = MatchLen::new(match_len);
        dst.write_quad(bytes, literal_len)?;
        write_match(dst, match_len, self.match_distance)?;
        Ok(literal_len.get())
    }

    /// # Safety
    ///
    /// * `literal_len <= Vn::MAX_LITERAL_LEN`
    /// * `match_len <= Vn::MAX_MATCH_LEN`
    /// * `match_distance <= Vn::MAX_MATCH_DISTANCE`
    /// * `match_len != 0`
    unsafe fn typ_d<O>(
        &mut self,
        dst: &mut O,
        src: &[u8],
        (literal_len, match_len, match_distance): (u32, u32, u32),
    ) -> crate::Result<u32>
    where
        O: LzWriter,
    {
        debug_assert!(literal_len <= Vn::MAX_LITERAL_LEN as u32);
        debug_assert!(match_len <= Vn::MAX_MATCH_LEN as u32);
        debug_assert!(match_distance <= Vn::MAX_MATCH_DISTANCE as u32);
        debug_assert_ne!(match_len, 0);
        if src.len() < literal_len as usize + 8 {
            return Err(crate::Error::PayloadUnderflow);
        }
        let bytes = src.as_ptr().cast::<u32>().read_unaligned();
        let literal_len = LiteralLen::new(literal_len);
        let match_len = MatchLen::new(match_len);
        let match_distance = MatchDistanceUnpack::new(match_distance);
        self.match_distance = match_distance;
        dst.write_quad(bytes, literal_len)?;
        write_match(dst, match_len, match_distance)?;
        Ok(literal_len.get())
    }
}

impl Default for VnCore {
    #[inline(always)]
    fn default() -> Self {
        Self { n_raw_bytes: 0, n_payload_bytes: 0, match_distance: MatchDistanceUnpack::default() }
    }
}

impl From<VnBlock> for VnCore {
    fn from(block: VnBlock) -> Self {
        Self {
            match_distance: MatchDistanceUnpack::default(),
            n_raw_bytes: block.n_raw_bytes(),
            n_payload_bytes: block.n_payload_bytes(),
        }
    }
}

// Isolating the normally inlined write_match to allow the compiler to decide on whether to inline
// or not.
fn write_match<O>(
    dst: &mut O,
    match_len: MatchLen<Vn>,
    match_distance: MatchDistanceUnpack<Vn>,
) -> crate::Result<()>
where
    O: LzWriter,
{
    dst.write_match(match_len, match_distance)
}
