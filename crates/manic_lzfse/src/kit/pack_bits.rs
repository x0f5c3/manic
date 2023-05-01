pub trait PackBits {
    fn get_bits(&self, offset: u32, nbits: u32) -> Self;

    fn set_bits(&mut self, offset: u32, nbits: u32, value: Self);
}

impl PackBits for u64 {
    #[inline(always)]
    fn get_bits(&self, offset: u32, nbits: u32) -> Self {
        debug_assert!(offset + nbits <= 64);
        debug_assert!(nbits <= 64);
        (self >> offset) & mask_u64(nbits)
    }

    #[inline(always)]
    fn set_bits(&mut self, offset: u32, nbits: u32, value: u64) {
        debug_assert!(offset + nbits <= 64);
        debug_assert!(nbits <= 64);
        debug_assert!(mask_u64(nbits) & value == value);
        *self |= value << offset;
    }
}

impl PackBits for u32 {
    #[inline(always)]
    fn get_bits(&self, offset: u32, nbits: u32) -> Self {
        debug_assert!(offset + nbits <= 32);
        debug_assert!(nbits <= 32);
        (self >> offset) & mask_u32(nbits)
    }

    #[inline(always)]
    fn set_bits(&mut self, offset: u32, nbits: u32, value: u32) {
        debug_assert!(offset + nbits <= 32);
        debug_assert!(nbits <= 32);
        debug_assert!(mask_u32(nbits) & value == value);
        *self |= value << offset;
    }
}

impl PackBits for u16 {
    #[inline(always)]
    fn get_bits(&self, offset: u32, nbits: u32) -> Self {
        debug_assert!(offset + nbits <= 16);
        debug_assert!(nbits <= 16);
        (self >> offset) & mask_u16(nbits)
    }

    #[inline(always)]
    fn set_bits(&mut self, offset: u32, nbits: u32, value: u16) {
        debug_assert!(offset + nbits <= 16);
        debug_assert!(nbits <= 16);
        debug_assert!(mask_u16(nbits) & value == value);
        *self |= value << offset;
    }
}

impl PackBits for u8 {
    #[inline(always)]
    fn get_bits(&self, offset: u32, nbits: u32) -> Self {
        debug_assert!(offset + nbits <= 8);
        debug_assert!(nbits <= 8);
        (self >> offset) & mask_u8(nbits)
    }

    #[inline(always)]
    fn set_bits(&mut self, offset: u32, nbits: u32, value: u8) {
        debug_assert!(offset + nbits <= 8);
        debug_assert!(nbits <= 8);
        debug_assert!(mask_u8(nbits) & value == value);
        *self |= value << offset;
    }
}

#[inline(always)]
pub fn mask_u64(nbits: u32) -> u64 {
    debug_assert!(nbits < 64);
    (1 << nbits) - 1
}

#[inline(always)]
pub fn mask_u32(nbits: u32) -> u32 {
    debug_assert!(nbits <= 31);
    (1 << nbits) - 1
}

#[inline(always)]
pub fn mask_u16(nbits: u32) -> u16 {
    debug_assert!(nbits <= 15);
    (1 << nbits) - 1
}

#[inline(always)]
pub fn mask_u8(nbits: u32) -> u8 {
    debug_assert!(nbits <= 7);
    (1 << nbits) - 1
}
