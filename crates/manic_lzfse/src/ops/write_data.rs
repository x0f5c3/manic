use crate::kit::WIDE;

use std::mem;

/// Write unsigned integers as little endian bytes. Buffer overflow behaviour undefined.
pub trait WriteData {
    #[inline(always)]
    fn write_u8(&mut self, v: u8) {
        unsafe { self.write_data(&v.to_le_bytes()) };
    }

    #[inline(always)]
    fn write_u16(&mut self, v: u16) {
        unsafe { self.write_data(&v.to_le_bytes()) };
    }

    #[inline(always)]
    fn write_u32(&mut self, v: u32) {
        unsafe { self.write_data(&v.to_le_bytes()) };
    }

    #[inline(always)]
    fn write_u64(&mut self, v: u64) {
        unsafe { self.write_data(&v.to_le_bytes()) };
    }

    #[inline(always)]
    fn write_u128(&mut self, v: u128) {
        unsafe { self.write_data(&v.to_le_bytes()) };
    }

    #[inline(always)]
    fn write_usize(&mut self, v: usize) {
        unsafe { self.write_data(&v.to_le_bytes()) };
    }

    /// # Safety
    ///
    /// * `src.len() <= WIDE`
    unsafe fn write_data(&mut self, src: &[u8]);
}

impl WriteData for &mut [u8] {
    #[inline(always)]
    unsafe fn write_data(&mut self, src: &[u8]) {
        // Overflows panic.
        debug_assert!(src.len() <= WIDE);
        let len = src.len();
        let split = mem::take(self).split_at_mut(len);
        split.0.copy_from_slice(src);
        *self = split.1;
    }
}

impl<T: WriteData + ?Sized> WriteData for &mut T {
    #[inline(always)]
    fn write_u8(&mut self, v: u8) {
        (**self).write_u8(v)
    }

    #[inline(always)]
    fn write_u16(&mut self, v: u16) {
        (**self).write_u16(v)
    }

    #[inline(always)]
    fn write_u32(&mut self, v: u32) {
        (**self).write_u32(v)
    }

    #[inline(always)]
    fn write_u64(&mut self, v: u64) {
        (**self).write_u64(v)
    }

    #[inline(always)]
    fn write_u128(&mut self, v: u128) {
        (**self).write_u128(v)
    }

    #[inline(always)]
    fn write_usize(&mut self, v: usize) {
        (**self).write_usize(v)
    }

    #[inline(always)]
    unsafe fn write_data(&mut self, src: &[u8]) {
        (**self).write_data(src)
    }
}
