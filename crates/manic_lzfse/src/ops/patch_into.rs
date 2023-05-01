use crate::types::Idx;

use super::pos::Pos;

pub trait PatchInto: Pos {
    /// Expose `len` bytes at `pos` allowing us to write directly into the writer.
    /// Bounds violations panic.
    /// Patching to greater than `i32::MAX` relative to `self.pos()` is undefined.
    ///

    ///
    /// * `S::SHORT_LIMIT <= Self::SHORT_LIMIT`
    #[must_use]
    fn patch_into(&mut self, pos: Idx, len: usize) -> &mut [u8];

    #[inline(always)]
    fn patch_bytes(&mut self, pos: Idx, bytes: &[u8]) {
        self.patch_into(pos, bytes.len()).copy_from_slice(bytes);
    }
}

impl PatchInto for Vec<u8> {
    #[inline(always)]
    fn patch_into(&mut self, pos: Idx, len: usize) -> &mut [u8] {
        let delta = self.pos() - pos;
        let position = (self.len() as isize - delta as isize) as usize;
        &mut self[position..position + len]
    }
}

impl<T: PatchInto + ?Sized> PatchInto for &mut T {
    #[inline(always)]
    fn patch_into(&mut self, pos: Idx, len: usize) -> &mut [u8] {
        (**self).patch_into(pos, len)
    }

    #[inline(always)]
    fn patch_bytes(&mut self, pos: Idx, bytes: &[u8]) {
        (**self).patch_bytes(pos, bytes)
    }
}
