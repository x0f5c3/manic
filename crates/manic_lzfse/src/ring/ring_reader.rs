use crate::kit::{ReadExtFully, WIDE};
use crate::ops::{Len, PeekData, Pos, Skip};
use crate::types::{ByteReader, Idx};

use super::object::Ring;
use super::ring_block::RingBlock;
use super::ring_size::RingSize;
use super::ring_view::RingView;

use std::io::{self, Read};
use std::ptr;

pub struct RingReader<'a, I, T> {
    ring: Ring<'a, T>,
    inner: I,
    head: Idx,
    tail: Idx,
    is_eof: bool,
}

impl<'a, I, T: RingBlock> RingReader<'a, I, T> {
    #[inline(always)]
    pub fn new(ring: Ring<'a, T>, inner: I) -> Self {
        assert!(0x0100 <= T::RING_BLK_SIZE);
        assert!(T::RING_BLK_SIZE <= T::RING_SIZE / 4);
        assert!(WIDE <= T::RING_LIMIT as usize);
        Self { ring, inner, head: Idx::default(), tail: Idx::default(), is_eof: false }
    }

    pub fn into_inner(self) -> I {
        self.inner
    }

    #[inline(always)]
    fn fill_blk_len(&self) -> usize {
        (self.fill_len() / T::RING_BLK_SIZE as usize) * T::RING_BLK_SIZE as usize
    }

    #[inline(always)]
    fn fill_len(&self) -> usize {
        debug_assert!((self.tail - self.head) as u32 <= T::RING_SIZE);
        (self.head + T::RING_SIZE - self.tail) as usize
    }
}

impl<'a, I, T: RingSize> PeekData for RingReader<'a, I, T> {
    #[inline(always)]
    unsafe fn peek_data(&self, dst: &mut [u8]) {
        debug_assert!(dst.len() <= WIDE);
        debug_assert!(self.head <= self.tail);
        let index = self.head % T::RING_SIZE as usize;
        let len = dst.len();
        let src = self.ring.as_ptr().add(index);
        let dst = dst.as_mut_ptr();
        ptr::copy_nonoverlapping(src, dst, len);
    }
}

impl<'a, 'b, I: Read, T: 'a + Copy + RingBlock> ByteReader<'a> for RingReader<'b, I, T> {
    const VIEW_LIMIT: usize = T::RING_SIZE as usize - T::RING_BLK_SIZE as usize;

    type View = RingView<'a, T>;

    fn fill(&mut self) -> io::Result<()> {
        let mut len = self.fill_blk_len();
        while len != 0 && !self.is_eof {
            debug_assert_eq!(self.tail % T::RING_BLK_SIZE, 0);
            let index = self.tail % T::RING_SIZE as usize;
            let limit = T::RING_SIZE as usize - index;
            let m = len.min(limit);
            debug_assert_eq!(m % T::RING_BLK_SIZE as usize, 0);
            let mut buf = &mut self.ring[index..index + m];
            let n = self.inner.read_fully(&mut buf)?;
            if index == 0 {
                self.ring.head_copy_out();
            }
            len -= n;
            self.tail += n as u32;
            self.is_eof = n != m;
        }
        debug_assert!(self.ring.head_shadowed());
        Ok(())
    }

    /// Returns a head shadowed view.
    #[inline(always)]
    fn view(&'a self) -> Self::View {
        RingView::new(&self.ring, self.head, self.tail)
    }

    #[inline(always)]
    fn is_eof(&self) -> bool {
        self.is_eof
    }

    #[inline(always)]
    fn is_full(&self) -> bool {
        self.is_eof || self.fill_blk_len() == 0
    }
}

impl<'a, I, T> Pos for RingReader<'a, I, T> {
    #[inline(always)]
    fn pos(&self) -> Idx {
        self.head
    }
}

impl<'a, I, T> Skip for RingReader<'a, I, T> {
    #[inline(always)]
    unsafe fn skip_unchecked(&mut self, len: usize) {
        debug_assert!(len <= self.len());
        self.head += len as u32;
    }
}

impl<'a, I, T> Len for RingReader<'a, I, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        debug_assert!(self.head <= self.tail);
        (self.tail - self.head) as usize
    }
}

#[cfg(test)]
mod tests {
    use crate::ring::{RingBlock, RingBox, RingSize, RingType};
    use crate::types::ShortBuffer;

    use test_kit::Seq;

    use super::*;

    #[derive(Copy, Clone)]
    pub struct T;

    impl RingSize for T {
        const RING_SIZE: u32 = 0x4000;
    }

    impl RingType for T {
        const RING_LIMIT: u32 = 0x0100;
    }

    impl RingBlock for T {
        const RING_BLK_SIZE: u32 = 0x1000;
    }

    /// Basic `fill_len` and `fill_blk_len` boundary test.
    #[test]
    fn fill_len() {
        let mut core = RingBox::<T>::default();
        let mut rdr = RingReader::new((&mut core).into(), ());
        assert_eq!(rdr.fill_len(), T::RING_SIZE as usize);
        assert_eq!(rdr.fill_blk_len(), T::RING_SIZE as usize);
        rdr.tail += 1;
        assert_eq!(rdr.fill_len(), T::RING_SIZE as usize - 1);
        assert_eq!(rdr.fill_blk_len(), T::RING_SIZE as usize - T::RING_BLK_SIZE as usize);
    }

    /// Loop: fill and empty.
    #[test]
    #[ignore = "expensive"]
    fn seq_u8_1() -> io::Result<()> {
        let mut core = RingBox::<T>::default();
        let seq = Seq::default();
        let mut rdr = RingReader::new((&mut core).into(), seq);
        let mut seq = Seq::default();
        for _ in 0..0x10 {
            rdr.fill()?;
            let mut view = rdr.view();
            let view_len = view.len();
            assert_eq!(rdr.len(), view.len());
            assert!(RingReader::<Seq, T>::VIEW_LIMIT <= view_len);
            while view.len() != 0 {
                let bytes = view.short_bytes();
                let bytes_len = bytes.len();
                for &b in bytes.iter() {
                    assert_eq!(seq.next().unwrap(), b);
                }
                view.skip(bytes_len);
            }
            rdr.skip(view_len);
        }
        Ok(())
    }

    /// Loop: fill and take short byte segments.
    #[test]
    #[ignore = "expensive"]
    fn seq_u8_2() -> io::Result<()> {
        let mut core = RingBox::<T>::default();
        let seq = Seq::default();
        let mut rdr = RingReader::new((&mut core).into(), seq);
        let mut seq = Seq::default();
        for _ in 0..T::RING_SIZE * 2 {
            rdr.fill()?;
            let view = rdr.view();
            let view_len = view.len();
            assert_eq!(rdr.len(), view.len());
            assert!(RingReader::<Seq, T>::VIEW_LIMIT <= view_len);
            let bytes = view.short_bytes();
            let bytes_len = bytes.len();
            assert_eq!(bytes_len, T::RING_LIMIT as usize);
            for &b in bytes.iter() {
                assert_eq!(seq.next().unwrap(), b);
            }
            rdr.skip(bytes_len);
        }
        Ok(())
    }

    /// Loop: fill and take overlapping short byte segments.
    #[test]
    #[ignore = "expensive"]
    fn seq_u8_3() -> io::Result<()> {
        let mut core = RingBox::<T>::default();
        let seq = Seq::default();
        let mut rdr = RingReader::new((&mut core).into(), seq);
        let mut seq = Seq::default();
        for _ in 0..T::RING_SIZE * 2 {
            rdr.fill()?;
            let view = rdr.view();
            let view_len = view.len();
            assert_eq!(rdr.len(), view.len());
            assert!(RingReader::<Seq, T>::VIEW_LIMIT <= view_len);
            let buf = view.short_bytes();
            let buf_len = buf.len();
            assert_eq!(buf_len, T::RING_LIMIT as usize);
            {
                let mut seq = seq;
                for &b in buf.iter() {
                    assert_eq!(seq.next().unwrap(), b);
                }
            }
            seq.next().unwrap();
            rdr.skip(1);
        }
        Ok(())
    }

    // Loop 0..n: create &[..n] byte source, create reader and drain.
    #[test]
    #[ignore = "expensive"]
    fn seq_u8_4() -> io::Result<()> {
        let mut core = RingBox::<T>::default();
        for n in 0..T::RING_SIZE as usize * 2 {
            let seq = Read::take(Seq::default(), n as u64);
            let mut rdr = RingReader::new((&mut core).into(), seq);
            let mut seq = Seq::default();
            rdr.fill()?;
            assert_eq!(rdr.len(), n.min(T::RING_SIZE as usize));
            let mut view = rdr.view();
            assert_eq!(rdr.len(), view.len());
            while view.len() != 0 {
                let buf = view.short_bytes();
                let buf_len = buf.len();
                for &b in buf.iter() {
                    assert_eq!(seq.next().unwrap(), b);
                }
                view.skip(buf_len);
            }
        }
        Ok(())
    }
}
