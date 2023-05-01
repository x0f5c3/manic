use super::len::Len;

use std::mem;

pub trait Skip: Len {
    #[inline(always)]
    fn skip(&mut self, len: usize) {
        assert!(len <= self.len());
        unsafe { self.skip_unchecked(len) }
    }

    /// Skip `len` bytes unchecked.
    ///
    /// # Safety
    ///
    /// * `len <= self.len()`
    unsafe fn skip_unchecked(&mut self, len: usize);
}

impl Skip for &[u8] {
    #[inline(always)]
    unsafe fn skip_unchecked(&mut self, len: usize) {
        debug_assert!(len <= self.len());
        *self = self.get(len..);
    }
}

impl Skip for &mut [u8] {
    #[inline(always)]
    unsafe fn skip_unchecked(&mut self, len: usize) {
        debug_assert!(len <= self.len());
        *self = mem::take(self).get_mut(len..);
    }
}

impl<T: Skip + ?Sized> Skip for &mut T {
    #[inline(always)]
    fn skip(&mut self, len: usize) {
        (**self).skip(len)
    }

    #[inline(always)]
    unsafe fn skip_unchecked(&mut self, len: usize) {
        (**self).skip_unchecked(len)
    }
}
