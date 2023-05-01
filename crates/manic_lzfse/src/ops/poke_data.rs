use crate::kit::WIDE;

/// Poke unsigned integers as little endian bytes truncated to remaining buffer bytes.
pub trait PokeData {
    #[inline(always)]
    fn poke_u8(&mut self, v: u8) {
        unsafe { self.poke_data(v.to_le_bytes().as_ref()) };
    }

    #[inline(always)]
    fn poke_u16(&mut self, v: u16) {
        unsafe { self.poke_data(v.to_le_bytes().as_ref()) };
    }

    #[inline(always)]
    fn poke_u32(&mut self, v: u32) {
        unsafe { self.poke_data(v.to_le_bytes().as_ref()) };
    }

    #[inline(always)]
    fn poke_u64(&mut self, v: u64) {
        unsafe { self.poke_data(v.to_le_bytes().as_ref()) };
    }

    #[inline(always)]
    fn poke_usize(&mut self, v: usize) {
        unsafe { self.poke_data(v.to_le_bytes().as_ref()) };
    }

    /// Truncated to remaining buffer bytes.
    ///
    /// # Safety
    ///
    /// * `src.len() <= WIDE`
    unsafe fn poke_data(&mut self, src: &[u8]);
}

impl PokeData for [u8] {
    #[inline(always)]
    unsafe fn poke_data(&mut self, src: &[u8]) {
        debug_assert!(src.len() <= WIDE);
        if src.len() <= self.len() {
            (&mut self[..src.len()]).copy_from_slice(src);
        } else {
            self.copy_from_slice(&src[..self.len()]);
        }
    }
}

impl<T: PokeData + ?Sized> PokeData for &mut T {
    #[inline(always)]
    fn poke_u8(&mut self, v: u8) {
        (**self).poke_u8(v)
    }

    #[inline(always)]
    fn poke_u16(&mut self, v: u16) {
        (**self).poke_u16(v)
    }

    #[inline(always)]
    fn poke_u32(&mut self, v: u32) {
        (**self).poke_u32(v)
    }

    #[inline(always)]
    fn poke_u64(&mut self, v: u64) {
        (**self).poke_u64(v)
    }

    #[inline(always)]
    fn poke_usize(&mut self, v: usize) {
        (**self).poke_usize(v)
    }

    #[inline(always)]
    unsafe fn poke_data(&mut self, src: &[u8]) {
        (**self).poke_data(src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8() {
        let mut bytes = [0x00; 2];
        bytes.as_mut().poke_u8(0x01);
        assert_eq!(bytes, [0x01, 0x00]);
    }

    #[test]
    fn u16() {
        let mut bytes = [0x00; 3];
        bytes.as_mut().poke_u16(0x0201);
        assert_eq!(bytes, [0x01, 0x02, 0x00]);
    }

    #[test]
    fn u16_edge() {
        let mut bytes = [0x00; 1];
        bytes.as_mut().poke_u16(0x0201);
        assert_eq!(bytes, [0x01]);
    }

    #[test]
    fn u32() {
        let mut bytes = [0x00; 5];
        bytes.as_mut().poke_u32(0x04030201);
        assert_eq!(bytes, [0x01, 0x02, 0x03, 0x04, 0x00]);
    }

    #[test]
    fn u32_edge() {
        let mut bytes = [0x00; 1];
        bytes.as_mut().poke_u32(0x04030201);
        assert_eq!(bytes, [0x01]);
    }

    #[test]
    fn u64() {
        let mut bytes = [0x00; 9];
        bytes.as_mut().poke_u64(0x0807060504030201);
        assert_eq!(bytes, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00]);
    }

    #[test]
    fn u64_edge() {
        let mut bytes = [0x00; 1];
        bytes.as_mut().poke_u64(0x0807060504030201);
        assert_eq!(bytes, [0x01]);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn usize() {
        let mut bytes = [0x00; 9];
        bytes.as_mut().poke_usize(0x0807060504030201);
        assert_eq!(bytes, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00]);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn usize_edge() {
        let mut bytes = [0x00; 1];
        bytes.as_mut().poke_usize(0x0807060504030201);
        assert_eq!(bytes, [0x01]);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn usize() {
        let mut bytes = [0x00; 5];
        bytes.as_mut().poke_usize(0x04030201);
        assert_eq!(bytes, [0x01, 0x02, 0x03, 0x04, 0x00]);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn usize_edge() {
        let mut bytes = [0x00; 1];
        bytes.as_mut().poke_usize(0x04030201);
        assert_eq!(bytes, [0x01]);
    }
}
