use crate::kit::WIDE;
use crate::ops::{Len, PokeData, Skip, WriteData};

use std::mem;
use std::ops::{Deref, DerefMut};
use std::slice;

/// `&mut [u8]` with at least `WIDE` slack bytes beyond the upper limit.
pub struct WideBytesMut<'a>(&'a mut [u8]);

impl<'a> WideBytesMut<'a> {
    #[allow(dead_code)]
    pub fn from_bytes(bytes: &mut [u8], len: usize) -> Self {
        assert!(len + WIDE <= bytes.len());
        unsafe { Self::from_bytes_unchecked(bytes, len) }
    }

    #[inline(always)]
    pub unsafe fn from_bytes_unchecked(bytes: &mut [u8], len: usize) -> Self {
        debug_assert!(len + WIDE <= bytes.len());
        Self::from_raw_parts(bytes.as_mut_ptr(), len)
    }

    /// # Safety
    ///
    /// `ptr` is valid for `len + WIDE` byte reads and writes.
    #[inline(always)]
    pub unsafe fn from_raw_parts(ptr: *mut u8, len: usize) -> Self {
        Self(slice::from_raw_parts_mut(ptr, len))
    }

    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

impl<'a> PokeData for WideBytesMut<'a> {
    #[inline(always)]
    unsafe fn poke_data(&mut self, src: &[u8]) {
        debug_assert!(src.len() <= WIDE);
        self.0.get_mut(..src.len()).copy_from_slice(src);
    }
}

impl<'a> WriteData for WideBytesMut<'a> {
    #[inline(always)]
    unsafe fn write_data(&mut self, src: &[u8]) {
        // Overflows panic.
        debug_assert!(src.len() <= WIDE);
        assert!(src.len() <= self.len());
        self.poke_data(src);
        self.skip_unchecked(src.len());
    }
}

impl<'a> Skip for WideBytesMut<'a> {
    #[inline(always)]
    unsafe fn skip_unchecked(&mut self, len: usize) {
        debug_assert!(len <= self.len());
        self.0 = mem::take(&mut self.0).get_mut(len..);
    }
}

impl<'a> Len for WideBytesMut<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> Deref for WideBytesMut<'a> {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> DerefMut for WideBytesMut<'a> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}
