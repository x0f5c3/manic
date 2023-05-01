use crate::kit::CopyType;
use crate::types::WideBytesMut;

use super::len::Len;
use super::short_limit::ShortLimit;
use super::skip::Skip;

use std::ptr;

pub trait CopyShort: Len + ShortLimit + Skip {
    #[inline(always)]
    fn read_short<V: CopyType>(&mut self, dst: WideBytesMut) {
        assert!(dst.len() <= Self::SHORT_LIMIT as usize);
        assert!(dst.len() <= self.len());
        unsafe { self.read_short_unchecked::<V>(dst) };
    }

    #[inline(always)]
    unsafe fn read_short_unchecked<V: CopyType>(&mut self, mut dst: WideBytesMut) {
        self.read_short_raw::<V>(dst.as_mut_ptr(), dst.len());
    }

    /// # Safety
    ///
    /// * `dst` is valid for `len + WIDE` byte writes.
    /// * `short_len <= ShortLimit::SHORT_LIMIT as usize`
    /// * `short_len <= Self::len()`
    #[inline(always)]
    unsafe fn read_short_raw<V: CopyType>(&mut self, dst: *mut u8, short_len: usize) {
        self.copy_short_raw::<V>(dst, short_len);
        self.skip_unchecked(short_len);
    }

    #[inline(always)]
    fn copy_short<V: CopyType>(&self, dst: WideBytesMut) {
        assert!(dst.len() <= self.len());
        unsafe { self.copy_short_unchecked::<V>(dst) };
    }

    #[inline(always)]
    unsafe fn copy_short_unchecked<V: CopyType>(&self, mut dst: WideBytesMut) {
        self.copy_short_raw::<V>(dst.as_mut_ptr(), dst.len())
    }

    /// # Safety
    ///
    /// * `dst` is valid for `len + WIDE` byte writes.
    /// * `short_len <= ShortLimit::SHORT_LIMIT as usize`
    /// * `short_len <= Self::len()`
    unsafe fn copy_short_raw<V: CopyType>(&self, dst: *mut u8, short_len: usize);
}

impl<T: CopyShort + ?Sized> CopyShort for &mut T {
    #[inline(always)]
    fn read_short<V: CopyType>(&mut self, dst: WideBytesMut) {
        (**self).read_short::<V>(dst)
    }

    #[inline(always)]
    unsafe fn read_short_unchecked<V: CopyType>(&mut self, dst: WideBytesMut) {
        (**self).read_short_unchecked::<V>(dst)
    }

    #[inline(always)]
    unsafe fn read_short_raw<V: CopyType>(&mut self, dst: *mut u8, short_len: usize) {
        (**self).read_short_raw::<V>(dst, short_len)
    }

    #[inline(always)]
    fn copy_short<V: CopyType>(&self, dst: WideBytesMut) {
        (**self).copy_short::<V>(dst)
    }

    #[inline(always)]
    unsafe fn copy_short_unchecked<V: CopyType>(&self, dst: WideBytesMut) {
        (**self).copy_short_unchecked::<V>(dst)
    }

    #[inline(always)]
    unsafe fn copy_short_raw<V: CopyType>(&self, dst: *mut u8, short_len: usize) {
        (**self).copy_short_raw::<V>(dst, short_len)
    }
}

impl CopyShort for &[u8] {
    #[inline(always)]
    unsafe fn copy_short_raw<V: CopyType>(&self, dst: *mut u8, short_len: usize) {
        debug_assert!(short_len <= self.len());
        ptr::copy_nonoverlapping(self.as_ptr(), dst, short_len);
    }
}

#[cfg(test)]
mod tests {
    use crate::kit::{CopyTypeIndex, WIDE};

    use super::*;

    #[test]
    fn test_seq() {
        let mut bytes = [0u8; 1 + WIDE];
        let vec: Vec<u8> = (0u8..=255).collect();
        let mut literals = vec.as_slice();
        for i in 0..=255 {
            let dst = unsafe { WideBytesMut::from_raw_parts(bytes.as_mut_ptr(), 1) };
            literals.read_short::<CopyTypeIndex>(dst);
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
            literals.read_short::<CopyTypeIndex>(dst);
            for j in 0..i {
                assert_eq!(bytes[j], j as u8);
            }
        }
    }
}
