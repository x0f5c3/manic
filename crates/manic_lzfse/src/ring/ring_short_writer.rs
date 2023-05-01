use crate::bits::BitDst;
use crate::kit::WIDE;
use crate::ops::{
    Allocate, CopyLong, Flush, FlushLimit, PatchInto, Pos, ShortLimit, Truncate, WriteData,
    WriteLong, WriteShort,
};
use crate::types::{Idx, ShortWriter};

use super::object::Ring;
use super::ring_block::RingBlock;
use super::ring_type::RingType;

use std::io::{self, prelude::*};
use std::mem;
use std::ptr;

pub struct RingShortWriter<'a, O, T> {
    ring: Ring<'a, T>,
    head: Idx,
    idx: Idx,
    n_payload_bytes: u64,
    inner: O,
}

// Implementation notes:
//
// Not a true ring buffer. There are corner cases with values being written and overwritten within
// the shadow zones that are expensive to manage with a cyclic implementation, instead we opt for a
// more linear approach.
//
// Internal component, we use `LIMIT` and hard assertions to enforce write limits.
//
// T::BLK_SIZE specifies flush write alignment:
//   `0` - invalid value.
//   `1` - disabled.
//   `n` - power of two block value, typically 4K or 8K
impl<'a, O, T: RingBlock> RingShortWriter<'a, O, T> {
    #[allow(clippy::assertions_on_constants)]
    pub fn new(ring: Ring<'a, T>, inner: O) -> Self {
        assert_ne!(T::RING_BLK_SIZE, 0);
        assert!(T::RING_BLK_SIZE < T::RING_SIZE / 4);
        assert!(WIDE <= T::RING_LIMIT as usize);
        Self { ring, inner, head: Idx::default(), idx: Idx::default(), n_payload_bytes: 0 }
    }
}

impl<'a, O: Write, T: RingBlock> RingShortWriter<'a, O, T> {
    pub fn into_inner(mut self) -> crate::Result<(O, u64)> {
        self.flush(true)?;
        Ok((self.inner, self.n_payload_bytes))
    }
}

impl<'a, O, T> Pos for RingShortWriter<'a, O, T> {
    #[inline(always)]
    fn pos(&self) -> Idx {
        self.idx
    }
}

impl<'a, O, T: RingBlock> FlushLimit for RingShortWriter<'a, O, T> {
    const FLUSH_LIMIT: u32 = T::RING_SIZE - T::RING_BLK_SIZE;
}

impl<'a, O: Write, T: RingBlock> Flush for RingShortWriter<'a, O, T> {
    fn flush(&mut self, hard: bool) -> crate::Result<()> {
        if Self::FLUSH_LIMIT < (self.idx - self.head) as u32 {
            return Err(crate::Error::BufferOverflow);
        }
        let index = usize::from(self.idx);
        assert!(index <= T::RING_SIZE as usize);
        let o_bytes = if hard { 0 } else { index % T::RING_BLK_SIZE as usize };
        let n_bytes = index - o_bytes;
        self.inner.write_all(&self.ring[..n_bytes])?;
        self.n_payload_bytes += n_bytes as u64;
        if hard {
            self.inner.flush()?;
        }
        self.ring.copy_within(n_bytes..n_bytes + o_bytes, 0);
        self.idx = Idx::new(o_bytes as u32);
        self.head = self.idx;
        Ok(())
    }
}

impl<'a, O, T: RingBlock> Allocate for RingShortWriter<'a, O, T> {
    #[inline(always)]
    fn allocate(&mut self, len: usize) -> io::Result<()> {
        // Largely lazy, we'll catch errors on flush.
        if len <= Self::FLUSH_LIMIT as usize {
            Ok(())
        } else {
            Err(io::ErrorKind::Other.into())
        }
    }

    #[inline(always)]
    fn is_allocated(&mut self, _: usize) -> bool {
        // Lazy, we'll catch errors on flush.
        true
    }
}

impl<'a, O, T: RingBlock> BitDst for RingShortWriter<'a, O, T> {
    #[inline(always)]
    unsafe fn push_bytes_unchecked(&mut self, bytes: usize, n_bytes: usize) {
        debug_assert!(n_bytes <= mem::size_of::<usize>());
        let index = self.idx % T::RING_SIZE as usize;
        self.ring.as_mut_ptr().add(index).cast::<usize>().write_unaligned(bytes.to_le());
        self.idx += n_bytes as u32;
    }

    #[inline(always)]
    fn finalize(&mut self) -> io::Result<()> {
        let index = usize::from(self.idx);
        assert!(index <= T::RING_SIZE as usize);
        Ok(())
    }
}

impl<'a, O, T> Truncate for RingShortWriter<'a, O, T> {
    #[inline(always)]
    fn truncate(&mut self, idx: Idx) {
        assert!(self.head <= idx);
        assert!(idx <= self.idx);
        self.idx = idx;
    }
}

impl<'a, O, T: RingType> WriteData for RingShortWriter<'a, O, T> {
    #[inline(always)]
    unsafe fn write_data(&mut self, src: &[u8]) {
        // Overflows caught on flush.
        debug_assert!(src.len() <= WIDE);
        let len = src.len();
        let index = self.idx % T::RING_SIZE as usize;
        let dst = self.ring.as_mut_ptr().add(index);
        let src = src.as_ptr();
        ptr::copy_nonoverlapping(src, dst, len);
        self.idx += len as u32;
    }
}

impl<'a, O, T: RingType> WriteLong for RingShortWriter<'a, O, T> {
    #[inline(always)]
    fn write_long<I: CopyLong>(&mut self, src: I) -> io::Result<()> {
        // Overflows caught on flush.
        let len = src.len();
        let index = self.idx % T::RING_SIZE as usize;
        if index + len <= T::RING_SIZE as usize {
            let dst = unsafe { self.ring.as_mut_ptr().add(index) };
            unsafe { src.copy_long_raw(dst, len) }
        }
        self.idx += len as u32;
        Ok(())
    }
}

impl<'a, O, T: RingType> PatchInto for RingShortWriter<'a, O, T> {
    fn patch_into(&mut self, pos: Idx, len: usize) -> &mut [u8] {
        assert!(len <= T::RING_LIMIT as usize);
        assert!(self.head <= pos);
        assert!(pos + len as u32 <= self.idx);
        let position = pos % T::RING_SIZE as usize;
        unsafe { self.ring.get_mut(position..position + len) }
    }
}

impl<'a, O, T: RingBlock> WriteShort for RingShortWriter<'a, O, T> {
    #[inline(always)]
    unsafe fn short_set(&mut self, len: u32) {
        debug_assert!(len <= Self::SHORT_LIMIT);
        self.idx += len;
    }

    #[inline(always)]
    unsafe fn short_ptr(&mut self) -> *mut u8 {
        let index = self.idx % T::RING_SIZE as usize;
        self.ring.as_mut_ptr().add(index)
    }
}

impl<'a, O, T: RingType> ShortLimit for RingShortWriter<'a, O, T> {
    const SHORT_LIMIT: u32 = T::RING_LIMIT;
}

impl<'a, O: Write, T: RingBlock + RingType> ShortWriter for RingShortWriter<'a, O, T> {}

#[cfg(test)]
mod tests {
    use crate::ring::{RingBlock, RingBox, RingSize, RingType};

    use super::*;

    use std::io;
    use std::iter::Iterator;
    use test_kit::Seq;

    #[derive(Copy, Clone, Debug)]
    pub struct T;

    impl RingSize for T {
        const RING_SIZE: u32 = 0x1000;
    }

    impl RingType for T {
        const RING_LIMIT: u32 = 0x0100;
    }

    impl RingBlock for T {
        const RING_BLK_SIZE: u32 = 0x0200;
    }

    #[test]
    fn write_long() -> crate::Result<()> {
        const LIMIT: u32 = RingShortWriter::<io::Sink, T>::FLUSH_LIMIT;
        let src = Iterator::take(Seq::default(), T::RING_SIZE as usize).collect::<Vec<_>>();
        let mut ring_box = RingBox::<T>::default();
        for delta in 0..T::RING_BLK_SIZE as usize {
            let mut bytes = src.as_slice();
            let ring = (&mut ring_box).into();
            let dst = Vec::<u8>::default();
            let mut wtr = RingShortWriter::new(ring, dst);
            wtr.write_long(&bytes[..delta])?;
            wtr.flush(false)?;
            bytes = &bytes[delta..];
            wtr.write_long(&bytes[..LIMIT as usize])?;
            let (dst, n) = wtr.into_inner()?;
            assert!(dst == src[..delta + LIMIT as usize]);
            assert_eq!(dst.len() as u64, n);
        }
        Ok(())
    }

    #[test]
    fn write_long_overflow() -> crate::Result<()> {
        const LIMIT: u32 = RingShortWriter::<io::Sink, T>::FLUSH_LIMIT;
        let src = Iterator::take(Seq::default(), LIMIT as usize + 1).collect::<Vec<_>>();
        let mut ring_box = RingBox::<T>::default();
        for delta in 0..T::RING_BLK_SIZE as usize {
            let ring = (&mut ring_box).into();
            let mut wtr = RingShortWriter::new(ring, io::sink());
            wtr.write_long(&src[..delta])?;
            wtr.flush(false)?;
            wtr.write_long(src.as_slice())?;
            assert!(wtr.flush(true).is_err());
        }
        Ok(())
    }

    #[test]
    fn write_data() -> crate::Result<()> {
        const LIMIT: u32 = RingShortWriter::<io::Sink, T>::FLUSH_LIMIT;
        let src = Iterator::take(Seq::default(), T::RING_SIZE as usize).collect::<Vec<_>>();
        let mut ring_box = RingBox::<T>::default();
        for delta in 0..T::RING_BLK_SIZE as usize {
            let mut bytes = src.as_slice();
            let ring = (&mut ring_box).into();
            let dst = Vec::<u8>::default();
            let mut wtr = RingShortWriter::new(ring, dst);
            wtr.write_long(&bytes[..delta])?;
            wtr.flush(false)?;
            bytes = &bytes[delta..];
            for index in (0..LIMIT as usize).step_by(WIDE) {
                unsafe { wtr.write_data(&bytes[index..index + WIDE]) };
            }
            let (dst, n) = wtr.into_inner()?;
            assert!(dst == src[..delta + LIMIT as usize]);
            assert_eq!(dst.len() as u64, n);
        }
        Ok(())
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn push() -> io::Result<()> {
        let mut ring_box = RingBox::<T>::default();
        let ring = (&mut ring_box).into();
        let dst = Vec::<u8>::default();
        let mut wtr = RingShortWriter::new(ring, dst);
        wtr.push_bytes(0xFFFF_FFFF_FFFF_FFFF, 0);
        wtr.push_bytes(0x0807_0605_0403_0201, 8);
        wtr.push_bytes(0xFF0F_0E0D_0C0B_0A09, 7);
        wtr.push_bytes(0xFFFF_1514_1312_1110, 6);
        wtr.push_bytes(0xFFFF_FF1A_1918_1716, 5);
        wtr.push_bytes(0xFFFF_FFFF_1E1D_1C1B, 4);
        wtr.push_bytes(0xFFFF_FFFF_FF21_201F, 3);
        wtr.push_bytes(0xFFFF_FFFF_FFFF_2322, 2);
        wtr.push_bytes(0xFFFF_FFFF_FFFF_FF24, 1);
        wtr.push_bytes(0xFFFF_FFFF_FFFF_FFFF, 0);
        wtr.finalize()?;
        let (dst, n) = wtr.into_inner()?;
        assert_eq!(dst.len() as u64, n);
        assert_eq!(
            dst,
            [
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
                0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C,
                0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24
            ]
        );
        Ok(())
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn push() -> io::Result<()> {
        let mut ring_box = RingBox::<T>::default();
        let ring = (&mut ring_box).into();
        let dst = Vec::<u8>::default();
        let mut wtr = RingShortWriter::new(ring, dst);
        wtr.push_bytes(0xFFFF_FFFF, 0);
        wtr.push_bytes(0x0403_0201, 4);
        wtr.push_bytes(0xFF07_0605, 3);
        wtr.push_bytes(0xFFFF_0908, 2);
        wtr.push_bytes(0xFFFF_FF0A, 1);
        wtr.push_bytes(0xFFFF_FFFF, 0);
        wtr.finalize()?;
        let (dst, n) = wtr.into_inner()?;
        assert_eq!(dst.len() as u64, n);
        assert_eq!(dst, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,]);
        Ok(())
    }
}
