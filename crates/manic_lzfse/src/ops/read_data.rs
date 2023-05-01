use std::mem;

/// Read unsigned integers from little endian bytes. Buffer overflows panic.
pub trait ReadData {
    #[inline(always)]
    fn read_u8(&mut self) -> u8 {
        let mut bytes = [0u8; mem::size_of::<u8>()];
        unsafe { self.read_data(&mut bytes) };
        u8::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn read_u16(&mut self) -> u16 {
        let mut bytes = [0u8; mem::size_of::<u16>()];
        unsafe { self.read_data(&mut bytes) };
        u16::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn read_u32(&mut self) -> u32 {
        let mut bytes = [0u8; mem::size_of::<u32>()];
        unsafe { self.read_data(&mut bytes) };
        u32::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn read_u64(&mut self) -> u64 {
        let mut bytes = [0u8; mem::size_of::<u64>()];
        unsafe { self.read_data(&mut bytes) };
        u64::from_le_bytes(bytes)
    }

    #[inline(always)]
    fn read_usize(&mut self) -> usize {
        let mut bytes = [0u8; mem::size_of::<usize>()];
        unsafe { self.read_data(&mut bytes) };
        usize::from_le_bytes(bytes)
    }

    /// # Safety
    ///
    /// * `dst.len() <= WIDE`
    unsafe fn read_data(&mut self, dst: &mut [u8]);
}

impl ReadData for &[u8] {
    #[inline(always)]
    unsafe fn read_data(&mut self, dst: &mut [u8]) {
        let len = dst.len();
        let split = self.split_at(len);
        dst.copy_from_slice(split.0);
        *self = split.1;
    }
}

impl<T: ReadData + ?Sized> ReadData for &mut T {
    #[inline(always)]
    fn read_u8(&mut self) -> u8 {
        (**self).read_u8()
    }

    #[inline(always)]
    fn read_u16(&mut self) -> u16 {
        (**self).read_u16()
    }

    #[inline(always)]
    fn read_u32(&mut self) -> u32 {
        (**self).read_u32()
    }

    #[inline(always)]
    fn read_u64(&mut self) -> u64 {
        (**self).read_u64()
    }

    #[inline(always)]
    fn read_usize(&mut self) -> usize {
        (**self).read_usize()
    }

    #[inline(always)]
    unsafe fn read_data(&mut self, dst: &mut [u8]) {
        (**self).read_data(dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8() {
        let mut bytes = [0x01, 0x02].as_ref();
        assert_eq!(bytes.read_u8(), 0x01);
        assert_eq!(bytes, &[0x02]);
    }

    #[test]
    fn u16() {
        let mut bytes = [0x01, 0x02, 0x03].as_ref();
        assert_eq!(bytes.read_u16(), 0x0201);
        assert_eq!(bytes, &[0x03]);
    }

    #[test]
    fn u32() {
        let mut bytes = [0x01, 0x02, 0x03, 0x04, 0x05].as_ref();
        assert_eq!(bytes.read_u32(), 0x04030201);
        assert_eq!(bytes, &[0x05]);
    }

    #[test]
    fn u64() {
        let mut bytes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09].as_ref();
        assert_eq!(bytes.read_u64(), 0x0807060504030201);
        assert_eq!(bytes, &[0x09]);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn usize() {
        let mut bytes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09].as_ref();
        assert_eq!(bytes.read_usize(), 0x0807060504030201);
        assert_eq!(bytes, &[0x09]);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn usize() {
        let mut bytes = [0x01, 0x02, 0x03, 0x04, 0x05].as_ref();
        assert_eq!(bytes.read_usize(), 0x04030201);
        assert_eq!(bytes, &[0x05]);
    }
}
