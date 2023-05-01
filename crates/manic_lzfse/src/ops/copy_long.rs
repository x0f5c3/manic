use crate::types::WideBytesMut;

use super::len::Len;
use super::skip::Skip;

use std::ptr;

/// Copy long: eager, high volume, higher latency.
pub trait CopyLong: Len + Skip {
    #[inline(always)]
    fn read_long(&mut self, dst: WideBytesMut) {
        assert!(dst.len() <= self.len());
        unsafe { self.read_long_unchecked(dst) };
    }

    #[inline(always)]
    unsafe fn read_long_unchecked(&mut self, mut dst: WideBytesMut) {
        self.read_long_raw(dst.as_mut_ptr(), dst.len());
    }

    /// # Safety
    ///
    /// * `dst` is valid for `len + WIDE` byte writes.
    /// * `len <= Self::len()`
    #[inline(always)]
    unsafe fn read_long_raw(&mut self, dst: *mut u8, len: usize) {
        self.copy_long_raw(dst, len);
        self.skip_unchecked(len);
    }

    #[inline(always)]
    fn copy_long(&self, dst: WideBytesMut) {
        assert!(dst.len() <= self.len());
        unsafe { self.copy_long_unchecked(dst) };
    }

    #[inline(always)]
    unsafe fn copy_long_unchecked(&self, mut dst: WideBytesMut) {
        self.copy_long_raw(dst.as_mut_ptr(), dst.len())
    }

    /// # Safety
    ///
    /// * `dst` is valid for `len + WIDE` byte writes.
    /// * `len <= Self::len()`
    unsafe fn copy_long_raw(&self, dst: *mut u8, len: usize);
}

impl<T: CopyLong + ?Sized> CopyLong for &mut T {
    #[inline(always)]
    fn read_long(&mut self, dst: WideBytesMut) {
        (**self).read_long(dst)
    }

    #[inline(always)]
    unsafe fn read_long_unchecked(&mut self, dst: WideBytesMut) {
        (**self).read_long_unchecked(dst)
    }

    #[inline(always)]
    unsafe fn read_long_raw(&mut self, dst: *mut u8, len: usize) {
        (**self).read_long_raw(dst, len)
    }

    #[inline(always)]
    fn copy_long(&self, dst: WideBytesMut) {
        (**self).copy_long(dst)
    }

    #[inline(always)]
    unsafe fn copy_long_unchecked(&self, dst: WideBytesMut) {
        (**self).copy_long_unchecked(dst)
    }

    #[inline(always)]
    unsafe fn copy_long_raw(&self, dst: *mut u8, len: usize) {
        (**self).copy_long_raw(dst, len)
    }
}

impl CopyLong for &[u8] {
    #[inline(always)]
    unsafe fn copy_long_raw(&self, dst: *mut u8, len: usize) {
        debug_assert!(len <= self.len());
        ptr::copy_nonoverlapping(self.as_ptr(), dst, len);
    }
}

#[cfg(test)]
mod tests {
    use crate::kit::WIDE;

    use super::*;

    #[test]
    fn test_seq() {
        let mut bytes = [0u8; 1 + WIDE];
        let vec: Vec<u8> = (0u8..=255).collect();
        let mut literals = vec.as_slice();
        for i in 0..=255 {
            let dst = unsafe { WideBytesMut::from_raw_parts(bytes.as_mut_ptr(), 1) };
            literals.read_long(dst);
            assert_eq!(bytes[0], i);
        }
    }

    #[allow(clippy::needless_range_loop)]
    #[test]
    fn test_inc() {
        let mut bytes = [0u8; 255 + WIDE];
        for i in 0..=255 {
            let vec: Vec<u8> = (0u8..=255).collect();
            let mut literals = vec.as_slice();
            let dst = unsafe { WideBytesMut::from_raw_parts(bytes.as_mut_ptr(), i) };
            literals.read_long(dst);
            for j in 0..i {
                assert_eq!(bytes[j], j as u8);
            }
        }
    }
}
