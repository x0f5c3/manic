use crate::kit::{CopyType, CopyTypeLong, W00, WIDE};
use crate::ops::{CopyLong, CopyShort, Len, ShortLimit, Skip};

use std::ops::Deref;
use std::slice;

/// Byte wrapper with at least `WIDE` slack bytes.
pub struct WideBytes<'a>(&'a [u8]);

impl<'a> WideBytes<'a> {
    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8], len: usize) -> Self {
        assert!(len + WIDE <= bytes.len());
        unsafe { Self::from_bytes_unchecked(bytes, len) }
    }

    #[inline(always)]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8], len: usize) -> Self {
        debug_assert!(len + WIDE <= bytes.len());
        Self::from_raw_parts(bytes.as_ptr(), len)
    }

    /// # Safety
    ///
    /// `ptr` is valid for `len + WIDE` byte reads.
    #[inline(always)]
    pub unsafe fn from_raw_parts(ptr: *const u8, len: usize) -> Self {
        Self(slice::from_raw_parts(ptr, len))
    }
}

impl<'a> CopyLong for WideBytes<'a> {
    #[inline(always)]
    unsafe fn copy_long_raw(&self, dst: *mut u8, len: usize) {
        debug_assert!(len <= self.len());
        CopyTypeLong::wide_copy::<W00>(self.0.as_ptr(), dst, len);
    }
}

impl<'a> CopyShort for WideBytes<'a> {
    #[inline(always)]
    unsafe fn copy_short_raw<V: CopyType>(&self, dst: *mut u8, len: usize) {
        debug_assert!(len <= self.len());
        V::wide_copy::<W00>(self.0.as_ptr(), dst, len);
    }
}

impl<'a> Len for WideBytes<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> ShortLimit for WideBytes<'a> {
    const SHORT_LIMIT: u32 = i32::MAX as u32;
}

impl<'a> Skip for WideBytes<'a> {
    #[inline(always)]
    unsafe fn skip_unchecked(&mut self, len: usize) {
        debug_assert!(len <= self.len());
        self.0 = self.0.get(len..);
    }
}

impl<'a> Deref for WideBytes<'a> {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
