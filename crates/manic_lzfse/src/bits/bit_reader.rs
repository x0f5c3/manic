use crate::Error;

use super::accum::{Accum, ACCUM_MAX};
use super::bit_src::BitSrc;

use std::mem;

#[cfg(target_pointer_width = "64")]
const MASK: [usize; 9] = [
    0x0000_0000_0000_0000,
    0x0000_0000_0000_00FF,
    0x0000_0000_0000_FFFF,
    0x0000_0000_00FF_FFFF,
    0x0000_0000_FFFF_FFFF,
    0x0000_00FF_FFFF_FFFF,
    0x0000_FFFF_FFFF_FFFF,
    0x00FF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
];

#[cfg(target_pointer_width = "32")]
const MASK: [usize; 5] = [0x0000_0000, 0x0000_00FF, 0x0000_FFFF, 0x00FF_FFFF, 0xFFFF_FFFF];

pub struct BitReader<T: BitSrc> {
    accum: Accum,
    inner: T,
}

impl<T: BitSrc> BitReader<T> {
    #[inline(always)]
    pub fn new(mut inner: T, off: usize) -> crate::Result<Self> {
        assert!(off <= 7);
        let accum;
        let accum_bits;
        if off == 0 {
            accum = inner.init_1();
            accum_bits = mem::size_of::<usize>() as isize * 8 - 8;
        } else {
            accum = inner.init_0();
            accum_bits = mem::size_of::<usize>() as isize * 8 - off as isize;
        };
        if accum >> accum_bits != 0 {
            Err(Error::BadBitStream)
        } else {
            let accum = Accum::new(accum, accum_bits);
            Ok(Self { inner, accum })
        }
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn into_inner(self) -> (T, Accum) {
        (self.inner, self.accum)
    }

    #[inline(always)]
    pub fn flush(&mut self) {
        debug_assert!(0 <= self.accum.bits);
        debug_assert!(self.accum.bits <= ACCUM_MAX);
        let n_bytes = (ACCUM_MAX - self.accum.bits) as usize / 8;
        let n_bits = n_bytes * 8;
        self.accum.u <<= n_bits;
        debug_assert!(n_bytes < mem::size_of::<usize>());
        self.accum.u |= unsafe { self.inner.pop_bytes(n_bytes) } & unsafe { MASK.get(n_bytes) };
        self.accum.bits += n_bits as isize;
        debug_assert!(0 <= self.accum.bits);
        debug_assert!(self.accum.bits <= ACCUM_MAX);
        debug_assert_eq!(self.accum.u >> self.accum.bits, 0);
    }

    /// # Safety
    ///
    /// * No more than `ACCUM_MAX` bits in total are pulled without flushing.
    #[inline(always)]
    pub fn pull(&mut self, n_bits: usize) -> usize {
        debug_assert!(n_bits <= 32);
        self.accum.bits -= n_bits as isize;
        let result = self.accum.u >> self.accum.bits;
        self.accum.mask();
        result
    }

    #[inline(always)]
    pub fn finalize(mut self) -> crate::Result<()> {
        self.flush();
        if self.inner.len() as isize + self.accum.bits / 8 < 8 {
            return Err(Error::PayloadUnderflow);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ops::Len;

    use test_kit::Fibonacci;

    use super::super::byte_bits::ByteBits;
    use super::*;

    // Bit stream of the first 32 Fibonacci numbers.
    const FIB_32_BS: [u8; 49] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7B, 0xB1, 0xAB, 0x78, 0x67, 0x21, 0xD3,
        0xF3, 0x8A, 0xB9, 0x7D, 0x8F, 0x31, 0xB4, 0x0A, 0xB6, 0x69, 0x61, 0xF5, 0xA5, 0x18, 0xFF,
        0x06, 0xA9, 0x8D, 0x28, 0x19, 0xA3, 0x5D, 0xE8, 0xDF, 0xB9, 0x6C, 0xD6, 0x62, 0x1F, 0x45,
        0x96, 0xBB, 0x15, 0x29,
    ];

    const FIB_32_OFF: usize = 2;

    #[test]
    fn fibonacci() -> crate::Result<()> {
        let src = ByteBits::new(FIB_32_BS.as_ref());
        let mut rdr = BitReader::new(src, FIB_32_OFF)?;
        let fib: Vec<u32> = Fibonacci::default().take(32).collect();
        for &v in fib.iter().rev() {
            rdr.flush();
            let u = unsafe { rdr.pull(32 - v.leading_zeros() as usize) as u32 };
            assert_eq!(u, v);
        }
        assert_eq!(rdr.inner.len() as isize + rdr.accum.bits / 8, 8);
        rdr.finalize()?;
        Ok(())
    }

    #[test]
    fn overflow() -> crate::Result<()> {
        let bytes = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
        let src = ByteBits::new(bytes.as_ref());
        for off in 0..7 {
            let mut rdr = BitReader::new(src, off)?;
            for _ in 0..8 - off {
                assert_eq!(unsafe { rdr.pull(1) }, 0);
            }
            assert_eq!(rdr.inner.len() as isize + rdr.accum.bits / 8, 8);
            assert_eq!(unsafe { rdr.pull(1) }, 1);
            assert_eq!(rdr.inner.len() as isize + rdr.accum.bits / 8, 7);
        }
        Ok(())
    }
}
