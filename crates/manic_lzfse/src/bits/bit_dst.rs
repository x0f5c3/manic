use crate::ops::{Allocate, Pos};

use std::io;
use std::mem;
use std::ptr;

/// BitWriter.
///
/// Memory must be allocated in advance via `Allocate`.
pub trait BitDst: Allocate + Pos {
    fn push_bytes(&mut self, bytes: usize, n_bytes: usize) {
        assert!(n_bytes <= mem::size_of::<usize>());
        unsafe { self.push_bytes_unchecked(bytes, n_bytes) }
    }

    /// Pushes bytes, as little-endian `usize` packed to the right with any unused bytes undefined.
    /// Usage after finalize undefined but not unsafe.
    ///
    /// # Panics
    ///
    /// Implementations may choose either to panic if insufficient memory is allocated or lazily
    /// throw an error on finalize.
    //
    /// # Safety
    ///
    /// * `n_bytes <= mem::size_of::<usize>()`
    unsafe fn push_bytes_unchecked(&mut self, bytes: usize, n_bytes: usize);

    fn finalize(&mut self) -> io::Result<()>;
}

impl<T: BitDst + ?Sized> BitDst for &mut T {
    #[inline(always)]
    unsafe fn push_bytes_unchecked(&mut self, bytes: usize, n_bytes: usize) {
        (**self).push_bytes_unchecked(bytes, n_bytes)
    }

    #[inline(always)]
    fn finalize(&mut self) -> io::Result<()> {
        (**self).finalize()
    }
}

impl BitDst for Vec<u8> {
    #[inline(always)]
    unsafe fn push_bytes_unchecked(&mut self, bytes: usize, n_bytes: usize) {
        debug_assert!(n_bytes <= mem::size_of::<usize>());
        let index = self.len();
        assert!(mem::size_of::<usize>() <= self.capacity() - self.len());
        let src = bytes.to_le_bytes().as_ptr();
        let dst = self.as_mut_ptr().add(index);
        ptr::copy_nonoverlapping(src, dst, mem::size_of::<usize>());
        self.set_len(index + n_bytes);
    }

    #[inline(always)]
    fn finalize(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn push() -> io::Result<()> {
        let mut bytes = Vec::with_capacity(0x002C);
        bytes.push_bytes(0xFFFF_FFFF_FFFF_FFFF, 0);
        bytes.push_bytes(0x0807_0605_0403_0201, 8);
        bytes.push_bytes(0xFF0F_0E0D_0C0B_0A09, 7);
        bytes.push_bytes(0xFFFF_1514_1312_1110, 6);
        bytes.push_bytes(0xFFFF_FF1A_1918_1716, 5);
        bytes.push_bytes(0xFFFF_FFFF_1E1D_1C1B, 4);
        bytes.push_bytes(0xFFFF_FFFF_FF21_201F, 3);
        bytes.push_bytes(0xFFFF_FFFF_FFFF_2322, 2);
        bytes.push_bytes(0xFFFF_FFFF_FFFF_FF24, 1);
        bytes.push_bytes(0xFFFF_FFFF_FFFF_FFFF, 0);
        bytes.finalize()?;
        assert_eq!(
            bytes,
            [
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
                0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C,
                0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24
            ]
        );
        Ok(())
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn push() -> io::Result<()> {
        let mut bytes = Vec::with_capacity(0x0011);
        bytes.push_bytes(0xFFFF_FFFF, 0);
        bytes.push_bytes(0x0403_0201, 4);
        bytes.push_bytes(0xFF07_0605, 3);
        bytes.push_bytes(0xFFFF_0908, 2);
        bytes.push_bytes(0xFFFF_FF0A, 1);
        bytes.push_bytes(0xFFFF_FFFF, 0);
        bytes.finalize()?;
        assert_eq!(bytes, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,]);
        Ok(())
    }
}
