use super::len::Len;

use std::mem;

pub trait Limit: Len {
    /// Limit to `len` bytes. If `self.len() <= len` this operation does nothing.
    fn limit(&mut self, len: usize);
}

impl Limit for &[u8] {
    #[inline(always)]
    fn limit(&mut self, len: usize) {
        let len = <[u8]>::len(self).min(len);
        *self = unsafe { self.get(..len) }
    }
}

impl Limit for &mut [u8] {
    #[inline(always)]
    fn limit(&mut self, len: usize) {
        let len = <[u8]>::len(self).min(len);
        *self = unsafe { mem::take(self).get_mut(..len) };
    }
}

impl<T: Limit + ?Sized> Limit for &mut T {
    #[inline(always)]
    fn limit(&mut self, len: usize) {
        (**self).limit(len)
    }
}
