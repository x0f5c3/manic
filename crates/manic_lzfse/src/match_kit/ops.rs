// Native trailing zero byte count.
#[inline(always)]
pub fn nctz_bytes(u: usize) -> u32 {
    #[cfg(target_endian = "little")]
    {
        u.leading_zeros() / 8
    }
    #[cfg(target_endian = "big")]
    {
        u.trailing_zeros() / 8
    }
}

// Native leading zero byte count.
#[inline(always)]
pub fn nclz_bytes(u: usize) -> u32 {
    #[cfg(target_endian = "little")]
    {
        u.trailing_zeros() / 8
    }
    #[cfg(target_endian = "big")]
    {
        u.leading_zeros() / 8
    }
}

#[cfg(target_pointer_width = "64")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_0() {
        let bytes = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let u = usize::from_ne_bytes(bytes);
        assert_eq!(nctz_bytes(u), 7);
        assert_eq!(nclz_bytes(u), 0);
    }

    #[test]
    fn bytes_1() {
        let bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let u = usize::from_ne_bytes(bytes);
        assert_eq!(nctz_bytes(u), 0);
        assert_eq!(nclz_bytes(u), 7);
    }
}

#[cfg(target_pointer_width = "32")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_0() {
        let bytes = [0x01, 0x00, 0x00, 0x00];
        let u = usize::from_ne_bytes(bytes);
        assert_eq!(nctz_bytes(u), 3);
        assert_eq!(nclz_bytes(u), 0);
    }

    #[test]
    fn bytes_1() {
        let bytes = [0x00, 0x00, 0x00, 0x01];
        let u = usize::from_ne_bytes(bytes);
        assert_eq!(nctz_bytes(u), 0);
        assert_eq!(nclz_bytes(u), 3);
    }
}
