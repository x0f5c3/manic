use super::accum::{Accum, ACCUM_MAX};
use super::bit_dst::BitDst;

use std::io;
use std::mem;

pub struct BitWriter<'a, T: BitDst> {
    accum: Accum,
    inner: &'a mut T,
}

impl<'a, T: BitDst> BitWriter<'a, T> {
    #[inline(always)]
    pub fn new(inner: &'a mut T, len: usize) -> io::Result<Self> {
        inner.allocate(len + mem::size_of::<usize>())?;
        Ok(Self { inner, accum: Accum::default() })
    }

    #[inline(always)]
    pub fn flush(&mut self) {
        debug_assert!(0 <= self.accum.bits);
        debug_assert!(self.accum.bits <= ACCUM_MAX);
        let n_bytes = self.accum.bits as usize / 8;
        unsafe { self.inner.push_bytes_unchecked(self.accum.u, n_bytes) };
        self.accum.u >>= n_bytes * 8;
        self.accum.bits -= n_bytes as isize * 8;
        debug_assert!(0 <= self.accum.bits);
        debug_assert!(self.accum.bits <= 7);
        debug_assert!(self.accum.u >> self.accum.bits == 0);
    }

    /// # Safety
    ///
    /// * No more than `ACCUM_MAX - 7` bits in total are pushed without flushing.
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, bits: usize, n_bits: usize) {
        debug_assert!(0 <= self.accum.bits + n_bits as isize);
        debug_assert!(self.accum.bits + n_bits as isize <= ACCUM_MAX);
        debug_assert!(bits >> n_bits == 0);
        self.accum.u |= bits << self.accum.bits;
        self.accum.bits += n_bits as isize;
    }

    #[inline(always)]
    pub fn finalize(mut self) -> io::Result<usize> {
        assert!(0 <= self.accum.bits);
        assert!(self.accum.bits <= ACCUM_MAX);
        let n_bytes = (self.accum.bits as usize + 7) / 8;
        unsafe { self.inner.push_bytes_unchecked(self.accum.u, n_bytes) };
        self.accum.bits -= n_bytes as isize * 8;
        debug_assert!(-7 <= self.accum.bits);
        debug_assert!(self.accum.bits <= 0);
        self.inner.finalize()?;
        Ok(-self.accum.bits as usize)
    }
}

#[cfg(test)]
mod tests {
    use test_kit::Fibonacci;

    use super::*;

    // Bit stream of the first 32 Fibonacci numbers.
    const FIB_32_BS: [u8; 41] = [
        0x7B, 0xB1, 0xAB, 0x78, 0x67, 0x21, 0xD3, 0xF3, 0x8A, 0xB9, 0x7D, 0x8F, 0x31, 0xB4, 0x0A,
        0xB6, 0x69, 0x61, 0xF5, 0xA5, 0x18, 0xFF, 0x06, 0xA9, 0x8D, 0x28, 0x19, 0xA3, 0x5D, 0xE8,
        0xDF, 0xB9, 0x6C, 0xD6, 0x62, 0x1F, 0x45, 0x96, 0xBB, 0x15, 0x29,
    ];

    const FIB_32_OFF: usize = 2;

    #[test]
    fn test_bit_writer_fibonacci() -> io::Result<()> {
        let mut vec = Vec::new();
        let mut wtr = BitWriter::new(&mut vec, FIB_32_BS.len())?;
        let fib: Vec<u32> = Fibonacci::default().take(32).collect();
        for &v in fib.iter() {
            wtr.flush();
            unsafe { wtr.push_unchecked(v as usize, 32 - v.leading_zeros() as usize) };
        }
        let off = wtr.finalize()?;
        assert_eq!(FIB_32_BS.as_ref(), vec.as_slice());
        assert_eq!(off, FIB_32_OFF);
        Ok(())
    }
}
