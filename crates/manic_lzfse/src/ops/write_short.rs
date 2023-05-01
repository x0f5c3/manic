use crate::kit::WIDE;
use crate::types::WideBytesMut;

use super::allocate::Allocate;
use super::short_limit::ShortLimit;

use std::io;
use std::ptr;
use std::slice;

/// Low-level write buffer access methods.
pub trait WriteShort: Allocate + ShortLimit {
    // Little endian.
    #[inline(always)]
    fn write_short_u8(&mut self, u: u8) -> io::Result<()> {
        self.write_short_bytes(&u.to_le_bytes())
    }

    // Little endian.
    #[inline(always)]
    fn write_short_u16(&mut self, u: u16) -> io::Result<()> {
        self.write_short_bytes(&u.to_le_bytes())
    }

    // Little endian.
    #[inline(always)]
    fn write_short_u32(&mut self, u: u32) -> io::Result<()> {
        self.write_short_bytes(&u.to_le_bytes())
    }

    // Little endian.
    #[inline(always)]
    fn write_short_u64(&mut self, u: u64) -> io::Result<()> {
        self.write_short_bytes(&u.to_le_bytes())
    }

    // Little endian.
    #[inline(always)]
    fn write_short_u128(&mut self, u: u128) -> io::Result<()> {
        self.write_short_bytes(&u.to_le_bytes())
    }

    // Little endian.
    #[inline(always)]
    fn write_short_usize(&mut self, u: usize) -> io::Result<()> {
        self.write_short_bytes(&u.to_le_bytes())
    }

    #[inline(always)]
    fn write_short_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        assert!(bytes.len() <= Self::SHORT_LIMIT as usize);
        let len = bytes.len();
        let src = bytes.as_ptr();
        self.allocate(len)?;
        let dst = unsafe { self.short_ptr() };
        unsafe { self.short_set(len as u32) };
        unsafe { ptr::copy_nonoverlapping(src, dst, len) };
        Ok(())
    }

    #[inline(always)]
    fn short_block(&mut self, len: u32) -> io::Result<&mut [u8]> {
        assert!(len <= Self::SHORT_LIMIT);
        unsafe { self.short_block_unchecked(len) }
    }

    /// Allocate and expose `len` byte block allowing us to write directly into the writer. Any
    /// allocated but unwritten block bytes remain undefined.
    ///
    /// # Assert
    ///
    /// * `len <= Self::SHORT_LIMIT`
    #[inline(always)]
    unsafe fn short_block_unchecked(&mut self, len: u32) -> io::Result<&mut [u8]> {
        debug_assert!(len <= Self::SHORT_LIMIT);
        self.allocate(len as usize)?;
        let ptr = self.short_ptr();
        self.short_set(len);
        Ok(slice::from_raw_parts_mut(ptr, len as usize))
    }

    #[inline(always)]
    fn short_wide_block(&mut self, len: u32) -> io::Result<WideBytesMut> {
        assert!(len <= Self::SHORT_LIMIT);
        unsafe { self.short_wide_block_unchecked(len) }
    }

    /// Allocate and expose `len` byte block as `WideBytesMut` allowing us to write directly into the
    /// writer. Any allocated but unwritten block bytes remain undefined.
    ///
    /// # Assert
    ///
    /// * `len <= Self::SHORT_LIMIT`
    #[inline(always)]
    unsafe fn short_wide_block_unchecked(&mut self, len: u32) -> io::Result<WideBytesMut> {
        debug_assert!(len <= Self::SHORT_LIMIT);
        let ptr = self.short_ptr();
        self.allocate(len as usize + WIDE)?;
        self.short_set(len);
        Ok(WideBytesMut::from_raw_parts(ptr, len as usize))
    }

    /// Set the allocated bytes size to `len`.
    ///
    /// # Safety
    ///
    /// * `len` does not exceed the allocated byte length.
    unsafe fn short_set(&mut self, len: u32);

    /// Raw mut pointer to allocated bytes.
    unsafe fn short_ptr(&mut self) -> *mut u8;
}

impl<T: WriteShort + ?Sized> WriteShort for &mut T {
    #[inline(always)]
    fn write_short_u8(&mut self, u: u8) -> io::Result<()> {
        (**self).write_short_u8(u)
    }

    #[inline(always)]
    fn write_short_u16(&mut self, u: u16) -> io::Result<()> {
        (**self).write_short_u16(u)
    }

    #[inline(always)]
    fn write_short_u32(&mut self, u: u32) -> io::Result<()> {
        (**self).write_short_u32(u)
    }

    #[inline(always)]
    fn write_short_u64(&mut self, u: u64) -> io::Result<()> {
        (**self).write_short_u64(u)
    }

    #[inline(always)]
    fn write_short_u128(&mut self, u: u128) -> io::Result<()> {
        (**self).write_short_u128(u)
    }

    #[inline(always)]
    fn write_short_usize(&mut self, u: usize) -> io::Result<()> {
        (**self).write_short_usize(u)
    }

    #[inline(always)]
    fn write_short_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        (**self).write_short_bytes(bytes)
    }

    #[inline(always)]
    fn short_block(&mut self, len: u32) -> io::Result<&mut [u8]> {
        (**self).short_block(len)
    }

    #[inline(always)]
    unsafe fn short_block_unchecked(&mut self, len: u32) -> io::Result<&mut [u8]> {
        (**self).short_block_unchecked(len)
    }

    #[inline(always)]
    fn short_wide_block(&mut self, len: u32) -> io::Result<WideBytesMut> {
        (**self).short_wide_block(len)
    }

    #[inline(always)]
    unsafe fn short_wide_block_unchecked(&mut self, len: u32) -> io::Result<WideBytesMut> {
        (**self).short_wide_block_unchecked(len)
    }

    #[inline(always)]
    unsafe fn short_set(&mut self, len: u32) {
        (**self).short_set(len)
    }

    #[inline(always)]
    unsafe fn short_ptr(&mut self) -> *mut u8 {
        (**self).short_ptr()
    }
}

impl WriteShort for Vec<u8> {
    #[inline(always)]
    unsafe fn short_set(&mut self, len: u32) {
        debug_assert!(len as usize <= i32::MAX as usize);
        debug_assert!(self.is_allocated(len as usize));
        let index = self.len();
        self.set_len(index + len as usize);
    }

    #[inline(always)]
    unsafe fn short_ptr(&mut self) -> *mut u8 {
        let index = self.len();
        self.as_mut_ptr().add(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_0() -> crate::Result<()> {
        let mut vec = vec![0, 1, 2, 3, 4];
        let bytes = vec.short_block(0)?;
        assert_eq!(bytes.len(), 0);
        assert_eq!(vec, vec![0, 1, 2, 3, 4]);
        Ok(())
    }

    #[test]
    fn vec_1() -> crate::Result<()> {
        let mut vec = vec![0, 1, 2, 3, 4];
        let bytes = vec.short_block(1)?;
        assert_eq!(bytes.len(), 1);
        bytes[0] = 5;
        assert_eq!(vec, vec![0, 1, 2, 3, 4, 5]);
        Ok(())
    }

    #[test]
    fn vec_2() -> crate::Result<()> {
        let mut vec = vec![0, 1, 2, 3, 4];
        let bytes = vec.short_block(1)?;
        bytes[0] = 5;
        let bytes = vec.short_block(2)?;
        bytes[0] = 6;
        bytes[1] = 7;
        assert_eq!(vec, vec![0, 1, 2, 3, 4, 5, 6, 7]);
        Ok(())
    }

    #[test]
    fn vec_3() -> crate::Result<()> {
        let mut vec = vec![0, 1, 2, 3, 4];
        vec.write_short_bytes(&[])?;
        assert_eq!(vec, vec![0, 1, 2, 3, 4]);
        Ok(())
    }

    #[test]
    fn vec_4() -> crate::Result<()> {
        let mut vec = vec![0, 1, 2, 3, 4];
        vec.write_short_bytes(&[5])?;
        assert_eq!(vec, vec![0, 1, 2, 3, 4, 5]);
        Ok(())
    }

    #[test]
    fn vec_5() -> crate::Result<()> {
        let mut vec = vec![0, 1, 2, 3, 4];
        vec.write_short_bytes(&[5])?;
        vec.write_short_bytes(&[6])?;
        assert_eq!(vec, vec![0, 1, 2, 3, 4, 5, 6]);
        Ok(())
    }
}
