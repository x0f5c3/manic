use crate::encode::Backend;
use crate::kit::{CopyTypeIndex, WIDE};
use crate::lmd::MatchDistance;
use crate::ops::CopyShort;
use crate::types::{Idx, ShortBuffer, ShortWriter};

use super::block::VnBlock;
use super::constants::*;
use super::object::Vn;
use super::opc;

use std::io;

// Working slack: max op len + max literal len + EOS tag len
const SLACK: u32 = 0x03 + 0x010F + 0x08;

fn n_allocate(len: u32) -> usize {
    // Assuming a functional front end that pushes literals correctly:
    // * literals, at worst, cost 1 byte plus an additional 1 byte per 4 literals.
    // * match runs are always cost neutral.
    VN_HEADER_SIZE as usize + (len as usize / 4) * 5 + 32 + SLACK as usize + WIDE
}

pub struct VnBackend {
    mark: Idx,
    match_distance: u32,
    n_literals: u32,
    n_match_bytes: u32,
}

/// VN backend.
///
/// Memory is allocated in advance for the entire block based on the worst scenario.
/// Pushing literals with a block length of less than 4 more than once may overflow our memory
/// allocation.
impl Backend for VnBackend {
    type Type = Vn;

    #[inline(always)]
    fn init<O: ShortWriter>(&mut self, dst: &mut O, len: Option<u32>) -> io::Result<()> {
        if let Some(u) = len {
            self.mark = dst.pos();
            self.match_distance = 0;
            self.n_literals = 0;
            self.n_match_bytes = 0;
            let n = n_allocate(u);
            dst.allocate(n)?;
            dst.write_short_bytes(&[0u8; VN_HEADER_SIZE as usize])?;
            Ok(())
        } else {
            Err(io::ErrorKind::Other.into())
        }
    }

    #[inline(always)]
    fn push_literals<I: ShortBuffer, O: ShortWriter>(
        &mut self,
        dst: &mut O,
        mut literals: I,
    ) -> io::Result<()> {
        assert!(I::SHORT_LIMIT >= 0x010F);
        self.n_literals += literals.len() as u32;
        while literals.len() >= 0x10 {
            let len = literals.len().min(0x10F) as u32;
            unsafe { lrg_l(dst, &mut literals, len) };
        }
        if literals.len() > 0 {
            let len = literals.len() as u32;
            unsafe { sml_l(dst, &mut literals, len) };
        }
        Ok(())
    }

    #[inline(always)]
    fn push_match<I: ShortBuffer, O: ShortWriter>(
        &mut self,
        dst: &mut O,
        mut literals: I,
        mut match_len: u32,
        match_distance: MatchDistance<Vn>,
    ) -> io::Result<()>
    where
        I: ShortBuffer,
    {
        assert!(I::SHORT_LIMIT >= 0x010F);
        let match_distance = match_distance.get();
        self.n_literals += literals.len() as u32;
        self.n_match_bytes += match_len;
        while literals.len() >= 0x10 {
            let len = literals.len().min(0x10F) as u32;
            unsafe { lrg_l(dst, &mut literals, len) };
        }
        if literals.len() >= 0x04 {
            let len = literals.len() as u32;
            unsafe { sml_l(dst, &mut literals, len) };
        }
        let literal_len = literals.len();
        let n = opc::match_len_x(literal_len as u32).min(match_len);
        match_len -= n;
        if match_distance == self.match_distance {
            if literal_len == 0 {
                unsafe { sml_m(dst, n) };
            } else {
                unsafe { pre_d(dst, &mut literals, literal_len as u32, n) };
            }
        } else if match_distance < 0x600 {
            unsafe { sml_d(dst, &mut literals, literal_len as u32, n, match_distance) };
        } else if match_distance >= 0x4000 || match_len == 0 || n + match_len > 0x22 {
            unsafe { lrg_d(dst, &mut literals, literal_len as u32, n, match_distance) };
        } else {
            unsafe { med_d(dst, &mut literals, literal_len as u32, n, match_distance) };
        }
        self.match_distance = match_distance;
        while match_len > 0x0F {
            let limit = match_len.min(0x10F);
            unsafe { lrg_m(dst, limit) };
            match_len -= limit;
        }
        if match_len > 0 {
            unsafe { sml_m(dst, match_len) };
        }
        Ok(())
    }

    #[inline(always)]
    fn finalize<O: ShortWriter>(&mut self, dst: &mut O) -> io::Result<()> {
        dst.write_short_u64(EOS as u64)?;
        let n_payload_bytes = (dst.pos() - self.mark) as u32 - VN_HEADER_SIZE;
        let buf = dst.patch_into(self.mark, VN_HEADER_SIZE as usize);
        let n_raw_bytes = self.n_literals + self.n_match_bytes;
        VnBlock::new(n_raw_bytes, n_payload_bytes).expect("internal error").store(buf);
        Ok(())
    }
}

impl Default for VnBackend {
    #[inline(always)]
    fn default() -> Self {
        Self { mark: Idx::default(), match_distance: 0, n_literals: 0, n_match_bytes: 0 }
    }
}

unsafe fn sml_l<I: CopyShort, O: ShortWriter>(dst: &mut O, src: &mut I, literal_len: u32) {
    l(dst, src, literal_len, opc::encode_sml_l(literal_len), 1)
}

unsafe fn lrg_l<I: CopyShort, O: ShortWriter>(dst: &mut O, src: &mut I, literal_len: u32) {
    l(dst, src, literal_len, opc::encode_lrg_l(literal_len), 2)
}

unsafe fn l<I: CopyShort, O: ShortWriter>(
    dst: &mut O,
    src: &mut I,
    literal_len: u32,
    opu: u32,
    op_len: u32,
) {
    debug_assert!(literal_len <= 0x10F);
    debug_assert!(literal_len as usize <= src.len());
    debug_assert!(op_len <= 3);
    assert!(dst.is_allocated(SLACK as usize + WIDE));
    let ptr = dst.short_ptr();
    ptr.cast::<u32>().write_unaligned(opu.to_le());
    let ptr = ptr.add(op_len as usize);
    src.read_short_raw::<CopyTypeIndex>(ptr, literal_len as usize);
    dst.short_set(op_len + literal_len);
}

unsafe fn sml_m<O: ShortWriter>(dst: &mut O, match_len: u32) {
    m(dst, opc::encode_sml_m(match_len), 1);
}

unsafe fn lrg_m<O: ShortWriter>(dst: &mut O, match_len: u32) {
    m(dst, opc::encode_lrg_m(match_len), 2);
}

unsafe fn m<O: ShortWriter>(dst: &mut O, opu: u32, op_len: u32) {
    debug_assert!(op_len <= 3);
    assert!(dst.is_allocated(SLACK as usize + WIDE));
    let ptr = dst.short_ptr();
    ptr.cast::<u32>().write_unaligned(opu.to_le());
    dst.short_set(op_len);
}

unsafe fn pre_d<I: ShortBuffer, O: ShortWriter>(
    dst: &mut O,
    src: &mut I,
    literal_len: u32,
    match_len: u32,
) {
    lmd(dst, src, literal_len, opc::encode_pre_d(literal_len, match_len), 1)
}

unsafe fn sml_d<I: ShortBuffer, O: ShortWriter>(
    dst: &mut O,
    src: &mut I,
    literal_len: u32,
    match_len: u32,
    match_distance: u32,
) {
    lmd(dst, src, literal_len, opc::encode_sml_d(literal_len, match_len, match_distance), 2)
}

unsafe fn med_d<I: ShortBuffer, O: ShortWriter>(
    dst: &mut O,
    src: &mut I,
    literal_len: u32,
    match_len: u32,
    match_distance: u32,
) {
    lmd(dst, src, literal_len, opc::encode_med_d(literal_len, match_len, match_distance), 3)
}

unsafe fn lrg_d<I: ShortBuffer, O: ShortWriter>(
    dst: &mut O,
    src: &mut I,
    literal_len: u32,
    match_len: u32,
    match_distance: u32,
) {
    lmd(dst, src, literal_len, opc::encode_lrg_d(literal_len, match_len, match_distance), 3)
}

unsafe fn lmd<I: ShortBuffer, O: ShortWriter>(
    dst: &mut O,
    src: &mut I,
    literal_len: u32,
    opu: u32,
    op_len: u32,
) {
    debug_assert!(literal_len <= 4);
    debug_assert!(literal_len as usize <= src.len());
    debug_assert!(op_len <= 3);
    assert!(dst.is_allocated(SLACK as usize + WIDE));
    let literal_bytes = src.peek_u32();
    let ptr = dst.short_ptr();
    ptr.cast::<u32>().write_unaligned(opu.to_le());
    let ptr = ptr.add(op_len as usize);
    ptr.cast::<u32>().write_unaligned(literal_bytes.to_le());
    dst.short_set(op_len + literal_len);
}
