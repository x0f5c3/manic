use crate::kit::{CopyType, CopyTypeLong, Width};
use crate::ops::{CopyLong, CopyShort, Len, ShortLimit, Skip};

use std::marker::PhantomData;
use std::ops::Deref;
use std::slice;

/// Byte wrapper with a a maximum length of `T::SHORT_LIMIT` and at least `W::WIDTH` slack bytes.
#[derive(Copy, Clone)]
pub struct ShortBytes<'a, T, W>(&'a [u8], PhantomData<T>, PhantomData<W>);

impl<'a, T: ShortLimit, W: Width> ShortBytes<'a, T, W> {
    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8], len: usize) -> Self {
        assert!(len <= T::SHORT_LIMIT as usize);
        assert!(len + W::WIDTH <= bytes.len());
        unsafe { Self::from_bytes_unchecked(bytes, len) }
    }

    #[inline(always)]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8], len: usize) -> Self {
        debug_assert!(len <= T::SHORT_LIMIT as usize);
        debug_assert!(len + W::WIDTH <= bytes.len());
        Self::from_raw_parts(bytes.as_ptr(), len)
    }

    /// # Safety
    ///
    /// `len <= T::SHORT_LIMIT`
    /// `ptr` is valid for `len + WIDE` byte reads.
    #[inline(always)]
    pub unsafe fn from_raw_parts(ptr: *const u8, len: usize) -> Self {
        debug_assert!(len <= T::SHORT_LIMIT as usize);
        Self(slice::from_raw_parts(ptr, len), PhantomData::default(), PhantomData::default())
    }
}

impl<'a, T, W: Width> CopyLong for ShortBytes<'a, T, W> {
    #[inline(always)]
    unsafe fn copy_long_raw(&self, dst: *mut u8, len: usize) {
        debug_assert!(len <= self.len());
        CopyTypeLong::wide_copy::<W>(self.0.as_ptr(), dst, len);
    }
}

impl<'a, T: ShortLimit, W: Width> CopyShort for ShortBytes<'a, T, W> {
    #[inline(always)]
    unsafe fn copy_short_raw<V: CopyType>(&self, dst: *mut u8, len: usize) {
        debug_assert!(len <= self.len());
        V::wide_copy::<W>(self.0.as_ptr(), dst, len);
    }
}

impl<'a, T, W> Len for ShortBytes<'a, T, W> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, T: ShortLimit, W> ShortLimit for ShortBytes<'a, T, W> {
    const SHORT_LIMIT: u32 = T::SHORT_LIMIT;
}

impl<'a, T, W> Skip for ShortBytes<'a, T, W> {
    #[inline(always)]
    unsafe fn skip_unchecked(&mut self, len: usize) {
        debug_assert!(len <= self.len());
        self.0 = self.0.get(len..);
    }
}

impl<'a, T, W> Deref for ShortBytes<'a, T, W> {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
