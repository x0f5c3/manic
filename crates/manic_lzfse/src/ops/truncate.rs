use crate::types::Idx;

use super::pos::Pos;

pub trait Truncate: Pos {
    /// Truncate to `idx`.
    /// Bounds violations panic.
    /// Truncating to greater than `i32::MAX` relative to `self.pos()` is undefined.
    fn truncate(&mut self, idx: Idx);
}

impl Truncate for Vec<u8> {
    fn truncate(&mut self, idx: Idx) {
        let delta = self.pos() - idx;
        let index = (self.len() as isize - delta as isize) as usize;
        assert!(index <= self.len());
        unsafe { self.set_len(index) };
    }
}

impl<T: Truncate + ?Sized> Truncate for &mut T {
    #[inline(always)]
    fn truncate(&mut self, idx: Idx) {
        (**self).truncate(idx)
    }
}
