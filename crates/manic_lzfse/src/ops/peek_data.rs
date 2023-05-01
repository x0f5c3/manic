use crate::kit::WIDE;

use std::mem;

/// Peek unsigned integers from little endian bytes with buffer overflow bytes undefined.
pub trait PeekData {
    #[inline(always)]
    fn peek_u8(&self) -> u8 {
        let mut bytes = [0u8; mem::size_of::<u8>()];
        unsafe { self.peek_data(&mut bytes) };
        u8::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn peek_u16(&self) -> u16 {
        let mut bytes = [0u8; mem::size_of::<u16>()];
        unsafe { self.peek_data(&mut bytes) };
        u16::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn peek_u32(&self) -> u32 {
        let mut bytes = [0u8; mem::size_of::<u32>()];
        unsafe { self.peek_data(&mut bytes) };
        u32::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn peek_u64(&self) -> u64 {
        let mut bytes = [0u8; mem::size_of::<u64>()];
        unsafe { self.peek_data(&mut bytes) };
        u64::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn peek_usize(&self) -> usize {
        let mut bytes = [0u8; mem::size_of::<usize>()];
        unsafe { self.peek_data(&mut bytes) };
        usize::from_le_bytes(bytes)
    }

    /// Overflow bytes are undefined.
    ///
    /// # Safety
    ///
    /// * `dst.len() <= WIDE`
    unsafe fn peek_data(&self, dst: &mut [u8]);
}

impl PeekData for [u8] {
    #[inline(always)]
    unsafe fn peek_data(&self, dst: &mut [u8]) {
        debug_assert!(dst.len() <= WIDE);
        if dst.len() <= self.len() {
            dst.copy_from_slice(&self[..dst.len()]);
        } else {
            (&mut dst[..self.len()]).copy_from_slice(self);
        }
    }
}

impl<T: PeekData + ?Sized> PeekData for &T {
    #[inline(always)]
    fn peek_u8(&self) -> u8 {
        (**self).peek_u8()
    }

    #[inline(always)]
    fn peek_u16(&self) -> u16 {
        (**self).peek_u16()
    }

    #[inline(always)]
    fn peek_u32(&self) -> u32 {
        (**self).peek_u32()
    }

    #[inline(always)]
    fn peek_u64(&self) -> u64 {
        (**self).peek_u64()
    }

    #[inline(always)]
    fn peek_usize(&self) -> usize {
        (**self).peek_usize()
    }

    #[inline(always)]
    unsafe fn peek_data(&self, dst: &mut [u8]) {
        (**self).peek_data(dst)
    }
}

impl<T: PeekData + ?Sized> PeekData for &mut T {
    #[inline(always)]
    fn peek_u8(&self) -> u8 {
        (**self).peek_u8()
    }

    #[inline(always)]
    fn peek_u16(&self) -> u16 {
        (**self).peek_u16()
    }

    #[inline(always)]
    fn peek_u32(&self) -> u32 {
        (**self).peek_u32()
    }

    #[inline(always)]
    fn peek_u64(&self) -> u64 {
        (**self).peek_u64()
    }

    #[inline(always)]
    fn peek_usize(&self) -> usize {
        (**self).peek_usize()
    }

    #[inline(always)]
    unsafe fn peek_data(&self, dst: &mut [u8]) {
        (**self).peek_data(dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8() {
        assert_eq!([0x01].as_ref().peek_u8(), 0x01);
    }

    #[test]
    fn u16() {
        assert_eq!([0x01, 0x02].as_ref().peek_u16(), 0x0201);
    }

    #[test]
    fn u16_edge() {
        assert_eq!([0x01].as_ref().peek_u16(), 0x01);
    }

    #[test]
    fn u32() {
        assert_eq!([0x01, 0x02, 0x03, 0x04].as_ref().peek_u32(), 0x04030201);
    }

    #[test]
    fn u32_edge() {
        assert_eq!([0x01].as_ref().peek_u32(), 0x01);
    }

    #[test]
    fn u64() {
        assert_eq!(
            [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08].as_ref().peek_u64(),
            0x0807060504030201
        );
    }

    #[test]
    fn u64_edge() {
        assert_eq!([0x01].as_ref().peek_u64(), 0x01);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn usize() {
        assert_eq!(
            [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08].as_ref().peek_usize(),
            0x0807060504030201
        );
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn usize_edge() {
        assert_eq!([0x01].as_ref().peek_usize(), 0x01);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn usize() {
        assert_eq!([0x01, 0x02, 0x03, 0x04].as_ref().peek_usize(), 0x04030201);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn usize_edge() {
        assert_eq!([0x01].as_ref().peek_usize(), 0x01);
    }
}
