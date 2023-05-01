use crate::error::Error;
use crate::kit::CopyTypeIndex;
use crate::kit::{Width, WIDE};
use crate::lmd::{DMax, LiteralLen, MMax, MatchDistanceUnpack, MatchLen, Quad};
use crate::lz::{self, LzWriter};
use crate::ops::{CopyLong, CopyShort, Pos, ShortLimit};
use crate::types::{Idx, ShortBytes};

use super::object::Ring;
use super::ring_type::RingType;

use std::io::Write;
use std::ptr;

/// Ring LZ output.
///
/// Operation undefined at `i64::MAX` and above bytes output.
pub struct RingLzWriter<'a, O, T> {
    ring: Ring<'a, T>,
    inner: O,
    index: u64,
}

impl<'a, O, T: RingType> RingLzWriter<'a, O, T> {
    pub fn new(ring: Ring<'a, T>, inner: O) -> Self {
        assert!(0x0020 <= T::RING_LIMIT);
        assert!(0x0100 <= T::RING_SIZE);
        Self { ring, inner, index: 0 }
    }

    pub fn copy(&self, mut dst: &mut [u8], mut idx: Idx) {
        debug_assert!(dst.len() < T::RING_SIZE as usize / 2);
        debug_assert!(((Idx::from(self.index) - idx) as u32) < T::RING_SIZE / 2);
        loop {
            let len = dst.len();
            let index = idx % T::RING_SIZE as usize;
            let limit = T::RING_SIZE as usize - index;
            let src = unsafe { self.ring.as_ptr().add(index) };
            if len <= limit {
                unsafe { ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), len) };
                break;
            }
            unsafe { ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), limit) };
            idx += limit as u32;
            dst = unsafe { dst.get_mut(limit as usize..) };
        }
    }
}

impl<'a, O: Write, T: RingType> RingLzWriter<'a, O, T> {
    #[inline(never)]
    fn flush(&mut self, len: usize) -> crate::Result<()> {
        self.inner.write_all(&self.ring)?;
        self.ring.head_copy_in_len(len as usize);
        self.ring.tail_copy_out();
        Ok(())
    }

    #[cold]
    pub fn into_inner(mut self) -> crate::Result<O> {
        if self.index >= i64::MAX as u64 {
            return Err(Error::BufferOverflow);
        }
        let index = self.index as u32 % T::RING_SIZE;
        let bytes = unsafe { &self.ring.get(..index as usize) };
        self.inner.write_all(bytes)?;
        Ok(self.inner)
    }
}

impl<'a, O, T: RingType> Pos for RingLzWriter<'a, O, T> {
    #[inline(always)]
    fn pos(&self) -> Idx {
        Idx::from(self.index)
    }
}

impl<'a, O, T: RingType> ShortLimit for RingLzWriter<'a, O, T> {
    const SHORT_LIMIT: u32 = T::RING_LIMIT;
}

impl<'a, O: Write, T: RingType> LzWriter for RingLzWriter<'a, O, T> {
    const MAX_MATCH_DISTANCE: u32 = T::RING_SIZE / 2;

    const MAX_MATCH_LEN: u32 = T::RING_LIMIT;

    fn write_bytes_long<U: CopyLong>(&mut self, mut bytes: U) -> crate::Result<()> {
        // Lmdy script:
        // println!("L:{}", bytes.len());
        loop {
            let len = bytes.len();
            let dst_index = self.index as usize % T::RING_SIZE as usize;
            let limit = T::RING_SIZE as usize - dst_index;
            let dst = unsafe { self.ring.as_mut_ptr().add(dst_index) };
            if len < limit {
                // Likely.
                unsafe { bytes.read_long_raw(dst, len) };
                self.index += len as u64;
                break;
            }
            unsafe { bytes.read_long_raw(dst, limit) };
            self.index += limit as u64;
            self.flush(WIDE)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn write_bytes_short<U: ShortLimit, W: Width>(
        &mut self,
        bytes: ShortBytes<U, W>,
    ) -> crate::Result<()> {
        assert!(U::SHORT_LIMIT as u32 <= Self::SHORT_LIMIT);
        // Lmdy script:
        // println!("L:{}", bytes.len());
        let len = bytes.len();
        let dst_index = self.index as usize % T::RING_SIZE as usize;
        self.index += len as u64;
        let dst = unsafe { self.ring.as_mut_ptr().add(dst_index) };
        unsafe { bytes.copy_short_raw::<CopyTypeIndex>(dst, len) };
        if dst_index + len >= T::RING_SIZE as usize {
            // Unlikely.
            self.flush(U::SHORT_LIMIT as usize)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn write_quad(&mut self, bytes: u32, len: LiteralLen<Quad>) -> crate::Result<()> {
        // Lmdy script:
        // println!("L:{}", len.get());
        let len = len.get();
        let index = self.index as u32 % T::RING_SIZE;
        self.index += len as u64;
        unsafe { self.ring.set_quad_index(index as usize, bytes) };
        if index + len >= T::RING_SIZE {
            // Unlikely.
            self.flush(4)?;
        }
        Ok(())
    }

    #[allow(clippy::absurd_extreme_comparisons)]
    #[inline(always)]
    fn write_match<U>(
        &mut self,
        len: MatchLen<U>,
        distance: MatchDistanceUnpack<U>,
    ) -> crate::Result<()>
    where
        U: DMax + MMax,
    {
        // Lmdy script:
        // println!("M:{}", len.get());
        // println!("D:{}", distance.get());
        assert!(U::MAX_MATCH_LEN as u32 <= Self::MAX_MATCH_LEN);
        assert!(U::MAX_MATCH_DISTANCE <= Self::MAX_MATCH_DISTANCE);
        let len = len.get();
        let distance = distance.get();
        if distance as u64 <= self.index {
            // Likely
            let dst_idx = Idx::from(self.index);
            self.index += len as u64;
            let dst_index = dst_idx % T::RING_SIZE as usize;
            let dst = unsafe { self.ring.as_mut_ptr().add(dst_index) };
            let dst_end = unsafe { dst.add(len as usize) };
            let src = unsafe { dst.sub(distance as usize) as *const u8 };
            // A, B, C, D branch predictability is data dependent.
            // The Snappy dataset generally favours A > B > C > D.
            if distance > dst_index as u32 + T::RING_LIMIT {
                // A
                unsafe { lz::write_match_16(src.add(T::RING_SIZE as usize), dst, dst_end) };
            } else if distance > 16 {
                // B
                unsafe { lz::write_match_16(src, dst, dst_end) };
            } else if distance > 8 {
                // C
                unsafe { lz::write_match_8(src, dst, dst_end, distance as usize) }
            } else if distance == 0 {
                // Unlikely
                return Err(Error::BadDValue);
            } else {
                // D
                unsafe { lz::write_match_x(src, dst, dst_end, distance as usize) };
            }
            if (dst_index + len as usize) < T::RING_SIZE as usize {
                // Likely.
                Ok(())
            } else {
                // Unlikely.
                self.flush(dst_index + len as usize - T::RING_SIZE as usize)
            }
        } else {
            // Unlikely
            Err(Error::BadDValue)
        }
    }

    #[inline(always)]
    fn n_raw_bytes(&self) -> u64 {
        self.index
    }
}

#[cfg(test)]
mod tests {
    use crate::kit::{Wide, WIDE};
    use crate::lz::LzWriter;
    use crate::ring::{RingBox, RingSize, RingType};

    use test_kit::{Cycle, Rng, Seq, Zeros};

    use super::*;

    use std::io;

    #[derive(Copy, Clone, Debug)]
    pub struct T1;

    impl RingSize for T1 {
        const RING_SIZE: u32 = 0x0200;
    }

    impl RingType for T1 {
        const RING_LIMIT: u32 = T1::RING_SIZE / 4;
    }

    #[derive(Copy, Clone, Debug)]
    pub struct V1;

    impl MMax for V1 {
        const MAX_MATCH_LEN: u16 = RingLzWriter::<io::Sink, T1>::MAX_MATCH_LEN as u16;
    }

    impl DMax for V1 {
        const MAX_MATCH_DISTANCE: u32 = RingLzWriter::<io::Sink, T1>::MAX_MATCH_DISTANCE;
    }

    #[derive(Copy, Clone, Debug)]
    pub struct T2;

    impl RingSize for T2 {
        const RING_SIZE: u32 = 0x0400;
    }

    impl RingType for T2 {
        const RING_LIMIT: u32 = T2::RING_SIZE / 4;
    }

    #[derive(Copy, Clone, Debug)]
    pub struct V2;

    impl MMax for V2 {
        const MAX_MATCH_LEN: u16 = RingLzWriter::<io::Sink, T2>::MAX_MATCH_LEN as u16;
    }

    impl DMax for V2 {
        const MAX_MATCH_DISTANCE: u32 = RingLzWriter::<io::Sink, T2>::MAX_MATCH_DISTANCE;
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_long() -> crate::Result<()> {
        let mut ring_box = RingBox::<T1>::default();
        let src = Seq::default().take(T1::RING_SIZE as usize * 0x10).collect::<Vec<_>>();
        let mut dst = Vec::<u8>::default();
        for len in 1..T1::RING_SIZE * 0x10 {
            let mut wtr = RingLzWriter::new((&mut ring_box).into(), &mut dst);
            let mut i = 0;
            while i + len as usize <= src.len() {
                let bytes = &src[i..i + len as usize];
                wtr.write_bytes_long(bytes)?;
                i += len as usize;
            }
            assert_eq!(wtr.n_raw_bytes(), i as u64);
            wtr.into_inner()?;
            assert!(src[..i] == dst[..i]);
            dst.clear();
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_long_fuzz() -> crate::Result<()> {
        let mut ring_box = RingBox::<T1>::default();
        let src = Seq::default().take(T1::RING_SIZE as usize * 0x1000).collect::<Vec<_>>();
        let mut dst = Vec::<u8>::default();
        let mut wtr = RingLzWriter::new((&mut ring_box).into(), &mut dst);
        let mut rng = Rng::default();
        let mut i = 0;
        while i < src.len() {
            let len = (rng.gen() as usize % T1::RING_SIZE as usize * 2).min(src.len() - i);
            let bytes = &src[i..i + len];
            wtr.write_bytes_long(bytes)?;
            i += len as usize;
        }
        assert_eq!(wtr.n_raw_bytes(), i as u64);
        wtr.into_inner()?;
        assert!(src == dst);
        dst.clear();
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_short_bytes() -> crate::Result<()> {
        let mut ring_box = RingBox::<T1>::default();
        let src = Seq::default().take(T1::RING_LIMIT as usize).collect::<Vec<_>>();
        let mut dst = Vec::<u8>::default();
        for len in 1..=T1::RING_LIMIT as usize {
            let mut wtr = RingLzWriter::new((&mut ring_box).into(), &mut dst);
            let mut i = 0;
            while i + len <= src.len() - WIDE {
                let bytes = &src[i..];
                let short_bytes = ShortBytes::<RingLzWriter<(), T1>, Wide>::from_bytes(bytes, len);
                wtr.write_bytes_short(short_bytes)?;
                i += len as usize;
            }
            assert_eq!(wtr.n_raw_bytes(), i as u64);
            wtr.into_inner()?;
            assert!(src[..i] == dst);
            dst.clear();
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_short_bytes_fuzz() -> crate::Result<()> {
        let mut ring_box = RingBox::<T1>::default();
        let src = Seq::default().take(T1::RING_SIZE as usize * 0x1000).collect::<Vec<_>>();
        let mut dst = Vec::<u8>::default();
        let mut wtr = RingLzWriter::new((&mut ring_box).into(), &mut dst);
        let mut rng = Rng::default();
        let mut i = 0;
        while i < src.len() - WIDE {
            let len = (rng.gen() as usize % T1::RING_LIMIT as usize).min(src.len() - WIDE - i);
            let bytes = &src[i..];
            let short_bytes = ShortBytes::<RingLzWriter<(), T1>, Wide>::from_bytes(bytes, len);
            wtr.write_bytes_short(short_bytes)?;
            i += len as usize;
        }
        assert_eq!(wtr.n_raw_bytes(), i as u64);
        wtr.into_inner()?;
        assert!(src[..i] == dst);
        dst.clear();
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_quad() -> crate::Result<()> {
        let mut ring_box = RingBox::<T1>::default();
        let src = Seq::default().take(T1::RING_SIZE as usize * 0x10).collect::<Vec<_>>();
        let mut dst = Vec::<u8>::default();
        for offset in 0..=3 {
            let offset_len = LiteralLen::new(offset);
            for len in 1..=4 {
                let literal_len = LiteralLen::new(len);
                let mut wtr = RingLzWriter::new((&mut ring_box).into(), &mut dst);
                let bytes = unsafe { src.as_ptr().cast::<u32>().read_unaligned() };
                let mut i = offset as usize;
                wtr.write_quad(bytes, offset_len)?;
                while i < src.len() - 4 {
                    let bytes = unsafe { src.as_ptr().add(i).cast::<u32>().read_unaligned() };
                    wtr.write_quad(bytes, literal_len)?;
                    i += len as usize;
                }
                assert_eq!(wtr.n_raw_bytes(), i as u64);
                wtr.into_inner()?;
                assert!(src[..i] == dst[..i]);
                dst.clear();
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_quad_fuzz() -> crate::Result<()> {
        let mut ring_box = RingBox::<T1>::default();
        let dst = Vec::<u8>::default();
        let src = Seq::default().take(T1::RING_SIZE as usize * 0x1000).collect::<Vec<_>>();
        let mut wtr = RingLzWriter::new((&mut ring_box).into(), dst);
        let mut rng = Rng::default();
        let mut i = 0;
        while i < src.len() - 4 {
            let bytes = unsafe { src.as_ptr().add(i).cast::<u32>().read_unaligned() };
            let len = rng.gen() % 5;
            let literal_len = LiteralLen::new(len);
            wtr.write_quad(bytes, literal_len)?;
            i += len as usize;
        }
        assert_eq!(wtr.n_raw_bytes(), i as u64);
        let dst = wtr.into_inner()?;
        assert!(src[..i] == dst[..i]);
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn lmd() -> crate::Result<()> {
        let mut ring_box = RingBox::<T1>::default();
        let pad = Seq::new(Rng::new(0)).take(T1::RING_SIZE as usize).collect::<Vec<_>>();
        let src = Seq::new(Rng::new(1)).take(T1::RING_SIZE as usize).collect::<Vec<_>>();
        let mut dst = Vec::<u8>::default();
        for offset in 0..=T1::RING_SIZE as usize {
            for match_len in 0..=RingLzWriter::<io::Sink, T1>::MAX_MATCH_LEN as usize {
                for literal_len in 0..=RingLzWriter::<io::Sink, T1>::MAX_MATCH_DISTANCE as usize {
                    let pad = &pad[..offset];
                    let bytes = &src[..literal_len];
                    for match_distance in 1..=literal_len {
                        let mut wtr = RingLzWriter::new((&mut ring_box).into(), &mut dst);
                        wtr.write_bytes_long(pad)?;
                        wtr.write_bytes_long(bytes)?;
                        wtr.write_match::<V1>(
                            MatchLen::new(match_len as u32),
                            MatchDistanceUnpack::new(match_distance as u32),
                        )?;
                        assert_eq!(
                            wtr.n_raw_bytes(),
                            offset as u64 + literal_len as u64 + match_len as u64
                        );
                        wtr.into_inner()?;
                        assert_eq!(dst.len(), offset + literal_len + match_len);
                        // Pad
                        assert!(&dst[..offset] == pad);
                        // Bytes
                        assert!(src[..literal_len] == dst[offset..offset + literal_len]);
                        // Match
                        let index = offset + literal_len;
                        let match_index = index - match_distance as usize;
                        let match_dst = match_index..match_index + match_len as usize;
                        let match_src = index..index + match_len as usize;
                        assert!(dst[match_dst] == dst[match_src]);
                        dst.clear();
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn lmd_fuzz() -> crate::Result<()> {
        let mut ring_box = RingBox::<T1>::default();
        let src = Seq::default().take(T1::RING_SIZE as usize * 2).collect::<Vec<_>>();
        let mut dst = Vec::<u8>::default();
        let mut dup = Vec::<u8>::default();
        for seed in 0..=0x8000 {
            let mut wtr = RingLzWriter::new((&mut ring_box).into(), &mut dst);
            let mut rng = Rng::new(seed);
            while dup.len() < T1::RING_SIZE as usize * 0x1000 {
                // literal_len: 0..=RING_SIZE
                let l = rng.gen() % (T1::RING_LIMIT + 1);
                // match_len: 0..=MAX_MATCH_LEN
                let m = rng.gen() % (RingLzWriter::<io::Sink, T1>::MAX_MATCH_LEN + 1);
                // match_distance: 1..=MAX_MATCH_DISTANCE.min(dup.len() + l)
                let d = (rng.gen() % RingLzWriter::<io::Sink, T1>::MAX_MATCH_DISTANCE + 1)
                    .min(dup.len() as u32 + l);
                // random offset literals
                let index = rng.gen() % T1::RING_SIZE;
                let bytes = &src[index as usize..index as usize + l as usize];
                dup.write_bytes_long(bytes)?;
                wtr.write_bytes_long(bytes)?;
                if d == 0 {
                    continue;
                }
                dup.write_match::<V1>(MatchLen::new(m as u32), MatchDistanceUnpack::new(d as u32))?;
                wtr.write_match::<V1>(MatchLen::new(m as u32), MatchDistanceUnpack::new(d as u32))?;
            }
            assert_eq!(wtr.n_raw_bytes(), dup.n_raw_bytes());
            wtr.into_inner()?;
            assert!(dst == dup);
            dst.clear();
            dup.clear();
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn match_distance_bounds() -> crate::Result<()> {
        let bytes = [0u8];
        let mut ring_box = RingBox::<T1>::default();
        let mut wtr = RingLzWriter::new((&mut ring_box).into(), Zeros);
        wtr.write_bytes_long(bytes.as_ref())?;
        for match_distance in 1..=RingLzWriter::<io::Sink, T1>::MAX_MATCH_DISTANCE {
            wtr.write_match::<V1>(
                MatchLen::new(1),
                MatchDistanceUnpack::new(match_distance as u32),
            )?;
        }
        wtr.into_inner()?;
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn match_distance_bounds_add_1() -> crate::Result<()> {
        let bytes = [0u8];
        let mut ring_box = RingBox::<T1>::default();
        let mut wtr = RingLzWriter::new((&mut ring_box).into(), Zeros);
        for match_distance in 1..=RingLzWriter::<io::Sink, T1>::MAX_MATCH_DISTANCE {
            match wtr.write_match::<V1>(
                MatchLen::new(1),
                MatchDistanceUnpack::new(match_distance as u32),
            ) {
                Err(Error::BadDValue) => {}
                _ => panic!(),
            };
            wtr.write_bytes_long(bytes.as_ref())?;
        }
        wtr.into_inner()?;
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn match_distance_zero() -> crate::Result<()> {
        let bytes = [0u8];
        let mut ring_box = RingBox::<T1>::default();
        let mut wtr = RingLzWriter::new((&mut ring_box).into(), Zeros);
        let match_len = MatchLen::new(1);
        let match_distance = MatchDistanceUnpack::new(0);
        for _ in 0..T1::RING_LIMIT {
            match wtr.write_match::<V1>(match_len, match_distance) {
                Err(Error::BadDValue) => {}
                _ => panic!(),
            };
            wtr.write_bytes_long(bytes.as_ref())?;
        }
        wtr.into_inner()?;
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_u32_8gb() -> crate::Result<()> {
        let mut ring_box = RingBox::<T1>::default();
        let mut wtr = RingLzWriter::new((&mut ring_box).into(), Cycle::default());
        let mut cycle = Cycle::default();
        let literal_len = LiteralLen::new(1);
        for _ in 0..0x2_0000_0000u64 {
            let bytes = cycle.next().unwrap() as u32 * 0x01010101;
            wtr.write_quad(bytes, literal_len)?;
        }
        assert_eq!(wtr.n_raw_bytes(), 0x2_0000_0000u64);
        wtr.into_inner()?;
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_8gb() -> crate::Result<()> {
        let mut bytes = [0u8];
        let mut ring_box = RingBox::<T1>::default();
        let mut wtr = RingLzWriter::new((&mut ring_box).into(), Cycle::default());
        let mut cycle = Cycle::default();
        for _ in 0..0x2_0000_0000u64 {
            bytes[0] = cycle.next().unwrap();
            wtr.write_bytes_long(bytes.as_ref())?;
        }
        assert_eq!(wtr.n_raw_bytes(), 0x2_0000_0000u64);
        wtr.into_inner()?;
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_match_8gb() -> crate::Result<()> {
        let bytes = Cycle::default().take(0x0100).collect::<Vec<_>>();
        let mut ring_box = RingBox::<T2>::default();
        let mut wtr = RingLzWriter::new((&mut ring_box).into(), Cycle::default());
        let match_len = MatchLen::new(0x0100);
        let match_distance = MatchDistanceUnpack::new(0x0100);
        wtr.write_bytes_long(bytes.as_ref())?;
        for _ in 0..0x0200_0000u64 {
            wtr.write_match::<V2>(match_len, match_distance)?;
        }
        assert_eq!(wtr.n_raw_bytes(), 0x2_0000_0100u64);
        wtr.into_inner()?;
        Ok(())
    }
}
