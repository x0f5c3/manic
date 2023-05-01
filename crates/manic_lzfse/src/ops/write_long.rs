use crate::kit::WIDE;

use super::allocate::Allocate;
use super::copy_long::CopyLong;

use std::io;

pub trait WriteLong {
    /// Write `src`.
    /// Lazy error checking.
    /// `src <= isize::MAX`
    fn write_long<I: CopyLong>(&mut self, src: I) -> io::Result<()>;
}

impl WriteLong for Vec<u8> {
    #[inline(always)]
    fn write_long<I: CopyLong>(&mut self, src: I) -> io::Result<()> {
        let len = src.len();
        let index = self.len();
        self.allocate(len + WIDE)?;
        let dst = unsafe { self.as_mut_ptr().add(index) };
        unsafe { src.copy_long_raw(dst, len) };
        unsafe { self.set_len(index + len) };
        Ok(())
    }
}

impl<T: WriteLong + ?Sized> WriteLong for &mut T {
    #[inline(always)]
    fn write_long<I: CopyLong>(&mut self, src: I) -> io::Result<()> {
        (**self).write_long(src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seq() -> crate::Result<()> {
        let seq: Vec<u8> = (0u8..=255).collect();
        let mut dst = Vec::default();
        for i in 0..=255 {
            let src = &seq[i..i + 1];
            dst.write_long(src)?;
        }
        assert_eq!(dst, seq);
        Ok(())
    }

    #[allow(clippy::needless_range_loop)]
    #[test]
    fn test_inc() -> crate::Result<()> {
        let seq: Vec<u8> = (0u8..=255).collect();
        for i in 0..=256 {
            let mut dst = Vec::default();
            let src = &seq[..i];
            dst.write_long(src)?;
            assert_eq!(dst, &seq[..i]);
        }
        Ok(())
    }
}
