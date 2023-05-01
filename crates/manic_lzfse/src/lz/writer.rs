use crate::kit::{CopyTypeIndex, Width, WIDE};
use crate::lmd::{DMax, LiteralLen, MMax, MatchDistanceUnpack, MatchLen, Quad};
use crate::ops::{CopyLong, ShortLimit};
use crate::types::ShortBytes;
use crate::{error::Error, ops::CopyShort};

use super::object;

use std::mem;

/// LZ77 type output stage. Data is reconstructed using literal bytes and matches.
pub trait LzWriter: ShortLimit {
    const MAX_MATCH_DISTANCE: u32;

    const MAX_MATCH_LEN: u32;

    /// Write match to a maximum of `Self::MAX_MATCH_LEN` and `Self::MAX_MATCH_DISTANCE`.
    /// Errors leave the the writer in an undefined state.
    fn write_match<T>(
        &mut self,
        len: MatchLen<T>,
        distance: MatchDistanceUnpack<T>,
    ) -> crate::Result<()>
    where
        T: DMax + MMax;

    /// Write bytes to a maximum of `usize::MAX`
    /// Errors leave the the writer in an undefined state.
    fn write_bytes_long<T: CopyLong>(&mut self, bytes: T) -> crate::Result<()>;

    /// Write bytes to a maximum of `Self::SHORT_LEN`
    /// Errors leave the the writer in an undefined state.
    fn write_bytes_short<T: ShortLimit, W: Width>(
        &mut self,
        bytes: ShortBytes<T, W>,
    ) -> crate::Result<()>;

    /// Write bytes to a maximum of 4. Bytes are supplied as native endian u32 values.
    /// Errors leave the the writer in an undefined state.
    fn write_quad(&mut self, bytes: u32, len: LiteralLen<Quad>) -> crate::Result<()>;

    /// The number of raw, that is decompressed, bytes.
    fn n_raw_bytes(&self) -> u64;
}

// Implementation notes:
//
// The `write_bytes_short`, `write_quad` and `write_match` methods are hot, being continually
// called from core decoder loops. These constrained lengths allow us to optimize buffer writes.
// Additionally implementations should optimize these methods to the Snappy dataset where low
// latency, low throughput code is more effective.
//
// The `write_byte_long` is considered a high latency, high throughput copy. Called infrequently
// for bulk operations.

impl<T: LzWriter + ?Sized> LzWriter for &mut T {
    const MAX_MATCH_DISTANCE: u32 = T::MAX_MATCH_DISTANCE;

    const MAX_MATCH_LEN: u32 = T::MAX_MATCH_LEN;

    #[inline(always)]
    fn write_bytes_long<U: CopyLong>(&mut self, bytes: U) -> crate::Result<()> {
        (**self).write_bytes_long(bytes)
    }

    #[inline(always)]
    fn write_bytes_short<U: ShortLimit, W: Width>(
        &mut self,
        bytes: ShortBytes<U, W>,
    ) -> crate::Result<()> {
        (**self).write_bytes_short(bytes)
    }

    #[inline(always)]
    fn write_quad(&mut self, bytes: u32, len: LiteralLen<Quad>) -> crate::Result<()> {
        (**self).write_quad(bytes, len)
    }

    #[inline(always)]
    fn write_match<U>(
        &mut self,
        len: MatchLen<U>,
        distance: MatchDistanceUnpack<U>,
    ) -> crate::Result<()>
    where
        U: DMax + MMax,
    {
        (**self).write_match(len, distance)
    }

    #[inline(always)]
    fn n_raw_bytes(&self) -> u64 {
        (**self).n_raw_bytes()
    }
}

impl LzWriter for Vec<u8> {
    const MAX_MATCH_DISTANCE: u32 = u32::MAX as u32;

    const MAX_MATCH_LEN: u32 = u32::MAX as u32;

    fn write_bytes_long<T: CopyLong>(&mut self, bytes: T) -> crate::Result<()> {
        let len = bytes.len();
        self.reserve(len + WIDE);
        let index = self.len();
        unsafe {
            bytes.copy_long_raw(self.as_mut_ptr().add(index), len);
            self.set_len(index + len as usize);
        }
        Ok(())
    }

    #[allow(clippy::absurd_extreme_comparisons)]
    #[inline(always)]
    fn write_bytes_short<T: ShortLimit, W: Width>(
        &mut self,
        bytes: ShortBytes<T, W>,
    ) -> crate::Result<()> {
        assert!(T::SHORT_LIMIT <= Self::SHORT_LIMIT);
        let len = bytes.len();
        self.reserve(len + WIDE);
        let index = self.len();
        unsafe {
            bytes.copy_short_raw::<CopyTypeIndex>(self.as_mut_ptr().add(index), len);
            self.set_len(index + len as usize);
        }
        Ok(())
    }

    #[inline(always)]
    fn write_quad(&mut self, bytes: u32, len: LiteralLen<Quad>) -> crate::Result<()> {
        let len = len.get();
        self.reserve(mem::size_of::<u32>());
        let index = self.len();
        unsafe {
            self.as_mut_ptr().add(index).cast::<u32>().write_unaligned(bytes);
            self.set_len(index + len as usize);
        }
        Ok(())
    }

    #[allow(clippy::absurd_extreme_comparisons)]
    #[inline(always)]
    fn write_match<T>(
        &mut self,
        len: MatchLen<T>,
        distance: MatchDistanceUnpack<T>,
    ) -> crate::Result<()>
    where
        T: DMax + MMax,
    {
        assert!(T::MAX_MATCH_LEN as u32 <= Self::MAX_MATCH_LEN);
        assert!(T::MAX_MATCH_DISTANCE <= Self::MAX_MATCH_DISTANCE);
        let len = len.get();
        let distance = distance.get() as usize;
        let dst_index = self.len();
        if distance as usize <= dst_index {
            // Likely
            let src_index = dst_index - distance;
            self.reserve(dst_index + len as usize + 32);
            unsafe { self.set_len(dst_index + len as usize) };
            let src = unsafe { self.as_ptr().add(src_index) };
            let dst = unsafe { self.as_mut_ptr().add(dst_index) };
            let dst_end = unsafe { dst.add(len as usize) };
            if distance > 16 {
                unsafe { object::write_match_16(src, dst, dst_end) };
            } else if distance > 8 {
                unsafe { object::write_match_8(src, dst, dst_end, distance) }
            } else if distance == 0 {
                // Unlikely
                return Err(Error::BadDValue);
            } else {
                unsafe { object::write_match_x(src, dst, dst_end, distance) };
            };
            Ok(())
        } else {
            // Unlikely
            Err(Error::BadDValue)
        }
    }

    #[inline(always)]
    fn n_raw_bytes(&self) -> u64 {
        Vec::<u8>::len(self) as u64
    }
}

#[cfg(test)]
mod tests {
    use crate::kit::Wide;
    use crate::lmd::LMax;

    use test_kit::{Rng, Seq};

    use super::*;

    #[derive(Copy, Clone, Debug)]
    pub struct V1;

    impl LMax for V1 {
        const MAX_LITERAL_LEN: u16 = u16::MAX;
    }

    impl MMax for V1 {
        const MAX_MATCH_LEN: u16 = 0x0100;
    }

    impl DMax for V1 {
        const MAX_MATCH_DISTANCE: u32 = 0x0100;
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_long() -> crate::Result<()> {
        let src = Seq::default().take(0x1000).collect::<Vec<_>>();
        let mut wtr = Vec::<u8>::default();
        for len in 1..0x1000 {
            let mut i = 0;
            while i + len as usize <= src.len() {
                let bytes = &src[i..i + len];
                wtr.write_bytes_long(bytes)?;
                i += len as usize;
            }
            assert_eq!(wtr.n_raw_bytes(), i as u64);
            assert!(src[..i] == wtr[..i]);
            wtr.clear();
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_long_fuzz() -> crate::Result<()> {
        let src = Seq::default().take(0x0010_0000).collect::<Vec<_>>();
        let mut wtr = Vec::<u8>::default();
        let mut rng = Rng::default();
        let mut i = 0;
        while i < src.len() {
            let len = (rng.gen() as usize % 0x200).min(src.len() - i);
            let bytes = &src[i..i + len];
            wtr.write_bytes_long(bytes)?;
            i += len as usize;
        }
        assert_eq!(wtr.n_raw_bytes(), i as u64);
        assert!(src == wtr);
        wtr.clear();
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_short() -> crate::Result<()> {
        let src = Seq::default().take(0x1000).collect::<Vec<_>>();
        let mut wtr = Vec::<u8>::default();
        for len in 1..0x1000 {
            let mut i = 0;
            while i + len <= src.len() - WIDE {
                let bytes = &src[i..];
                let short_bytes = ShortBytes::<LiteralLen<V1>, Wide>::from_bytes(bytes, len);
                wtr.write_bytes_short(short_bytes)?;
                i += len as usize;
            }
            assert_eq!(wtr.n_raw_bytes(), i as u64);
            assert!(src[..i] == wtr);
            wtr.clear();
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_short_fuzz() -> crate::Result<()> {
        let src = Seq::default().take(0x0010_0000).collect::<Vec<_>>();
        let mut wtr = Vec::<u8>::default();
        let mut rng = Rng::default();
        let mut i = 0;
        while i < src.len() - WIDE {
            let len = (rng.gen() as usize % V1::MAX_LITERAL_LEN as usize).min(src.len() - WIDE - i);
            let bytes = &src[i..];
            let short_bytes = ShortBytes::<LiteralLen<V1>, Wide>::from_bytes(bytes, len);
            wtr.write_bytes_short(short_bytes)?;
            i += len as usize;
        }
        assert_eq!(wtr.n_raw_bytes(), i as u64);
        assert!(src[..i] == wtr);
        wtr.clear();
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_quad() -> crate::Result<()> {
        let src = Seq::default().take(0x1000).collect::<Vec<_>>();
        let mut wtr = Vec::default();
        for offset in 0..=3 {
            let offset_len = LiteralLen::new(offset);
            for len in 1..=4 {
                let literal_len = LiteralLen::new(len);
                let bytes = unsafe { src.as_ptr().cast::<u32>().read_unaligned() };
                let mut i = offset as usize;
                wtr.write_quad(bytes, offset_len)?;
                while i < src.len() - 4 {
                    let bytes = unsafe { src.as_ptr().add(i).cast::<u32>().read_unaligned() };
                    wtr.write_quad(bytes, literal_len)?;
                    i += len as usize;
                }
                assert_eq!(wtr.n_raw_bytes(), i as u64);
                assert!(src[..i] == wtr[..i]);
                wtr.clear();
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn write_bytes_u32_fuzz() -> crate::Result<()> {
        let src = Seq::default().take(0x0010_0000).collect::<Vec<_>>();
        let mut wtr = Vec::default();
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
        assert!(src[..i] == wtr[..i]);
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn lmd() -> crate::Result<()> {
        let pad = Seq::new(Rng::new(0)).take(0x0100).collect::<Vec<_>>();
        let src = Seq::new(Rng::new(1)).take(0x0100).collect::<Vec<_>>();
        let mut wtr = Vec::default();
        for offset in 0..0x0010 {
            for match_len in 0..=0x0100 {
                for literal_len in 0..=0x0100 {
                    let pad = &pad[..offset];
                    let bytes = &src[..literal_len];
                    for match_distance in 1..=literal_len {
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
                        assert_eq!(wtr.len(), offset + literal_len + match_len);
                        // Pad
                        assert!(&wtr[..offset] == pad);
                        // Bytes
                        assert!(src[..literal_len] == wtr[offset..offset + literal_len]);
                        // Match
                        let index = offset + literal_len;
                        let match_index = index - match_distance as usize;
                        let match_dst = match_index..match_index + match_len;
                        let match_src = index..index + match_len;
                        assert_eq!(wtr[match_dst], wtr[match_src]);
                        wtr.clear();
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn match_distance_bounds() -> crate::Result<()> {
        let bytes = [0u8];
        let mut wtr = Vec::default();
        wtr.write_bytes_long(bytes.as_ref())?;
        for match_distance in 1..V1::MAX_MATCH_DISTANCE {
            wtr.write_match::<V1>(
                MatchLen::new(1),
                MatchDistanceUnpack::new(match_distance as u32),
            )?;
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn match_distance_bounds_add_1() -> crate::Result<()> {
        let bytes = [0u8];
        let mut wtr = Vec::default();
        for match_distance in 1..V1::MAX_MATCH_DISTANCE {
            match wtr.write_match::<V1>(
                MatchLen::new(1),
                MatchDistanceUnpack::new(match_distance as u32),
            ) {
                Err(Error::BadDValue) => {}
                _ => panic!(),
            };
            wtr.write_bytes_long(bytes.as_ref())?;
        }
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn match_distance_zero() -> crate::Result<()> {
        let bytes = [0u8];
        let mut wtr = Vec::default();
        let match_len = MatchLen::new(1);
        let match_distance = MatchDistanceUnpack::new(0);
        for _ in 0..0x1_0000 {
            match wtr.write_match::<V1>(match_len, match_distance) {
                Err(Error::BadDValue) => {}
                _ => panic!(),
            };
            wtr.write_bytes_long(bytes.as_ref())?;
        }
        Ok(())
    }
}
