use std::mem;

pub const ACCUM_MAX: isize = mem::size_of::<usize>() as isize * 8 - 1;

#[derive(Copy, Clone)]
pub struct Accum {
    pub(super) u: usize,
    pub(super) bits: isize,
}

impl Accum {
    #[inline(always)]
    pub fn new(u: usize, bits: isize) -> Self {
        Self { u, bits }
    }

    #[inline(always)]
    pub fn mask(&mut self) {
        debug_assert!(self.bits as usize <= ACCUM_MASK.len());
        self.u &= ACCUM_MASK.get(self.bits as usize).unwrap();
    }

    // #[inline(always)]
    // pub unsafe fn mask(&mut self) {
    //     debug_assert!(self.bits as usize <= ACCUM_MASK.len());
    //     self.u &= (1 << self.bits) - 1;
    // }
}

impl Default for Accum {
    #[inline(always)]
    fn default() -> Self {
        Self { u: 0, bits: 0 }
    }
}

#[cfg(target_pointer_width = "64")]
pub const ACCUM_MASK: [usize; 65] = [
    0x0000_0000_0000_0000,
    0x0000_0000_0000_0001,
    0x0000_0000_0000_0003,
    0x0000_0000_0000_0007,
    0x0000_0000_0000_000F,
    0x0000_0000_0000_001F,
    0x0000_0000_0000_003F,
    0x0000_0000_0000_007F,
    0x0000_0000_0000_00FF,
    0x0000_0000_0000_01FF,
    0x0000_0000_0000_03FF,
    0x0000_0000_0000_07FF,
    0x0000_0000_0000_0FFF,
    0x0000_0000_0000_1FFF,
    0x0000_0000_0000_3FFF,
    0x0000_0000_0000_7FFF,
    0x0000_0000_0000_FFFF,
    0x0000_0000_0001_FFFF,
    0x0000_0000_0003_FFFF,
    0x0000_0000_0007_FFFF,
    0x0000_0000_000F_FFFF,
    0x0000_0000_001F_FFFF,
    0x0000_0000_003F_FFFF,
    0x0000_0000_007F_FFFF,
    0x0000_0000_00FF_FFFF,
    0x0000_0000_01FF_FFFF,
    0x0000_0000_03FF_FFFF,
    0x0000_0000_07FF_FFFF,
    0x0000_0000_0FFF_FFFF,
    0x0000_0000_1FFF_FFFF,
    0x0000_0000_3FFF_FFFF,
    0x0000_0000_7FFF_FFFF,
    0x0000_0000_FFFF_FFFF,
    0x0000_0001_FFFF_FFFF,
    0x0000_0003_FFFF_FFFF,
    0x0000_0007_FFFF_FFFF,
    0x0000_000F_FFFF_FFFF,
    0x0000_001F_FFFF_FFFF,
    0x0000_003F_FFFF_FFFF,
    0x0000_007F_FFFF_FFFF,
    0x0000_00FF_FFFF_FFFF,
    0x0000_01FF_FFFF_FFFF,
    0x0000_03FF_FFFF_FFFF,
    0x0000_07FF_FFFF_FFFF,
    0x0000_0FFF_FFFF_FFFF,
    0x0000_1FFF_FFFF_FFFF,
    0x0000_3FFF_FFFF_FFFF,
    0x0000_7FFF_FFFF_FFFF,
    0x0000_FFFF_FFFF_FFFF,
    0x0001_FFFF_FFFF_FFFF,
    0x0003_FFFF_FFFF_FFFF,
    0x0007_FFFF_FFFF_FFFF,
    0x000F_FFFF_FFFF_FFFF,
    0x001F_FFFF_FFFF_FFFF,
    0x003F_FFFF_FFFF_FFFF,
    0x007F_FFFF_FFFF_FFFF,
    0x00FF_FFFF_FFFF_FFFF,
    0x01FF_FFFF_FFFF_FFFF,
    0x03FF_FFFF_FFFF_FFFF,
    0x07FF_FFFF_FFFF_FFFF,
    0x0FFF_FFFF_FFFF_FFFF,
    0x1FFF_FFFF_FFFF_FFFF,
    0x3FFF_FFFF_FFFF_FFFF,
    0x7FFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
];

#[cfg(target_pointer_width = "32")]
pub const ACCUM_MASK: [usize; 33] = [
    0x0000_0000,
    0x0000_0001,
    0x0000_0003,
    0x0000_0007,
    0x0000_000F,
    0x0000_001F,
    0x0000_003F,
    0x0000_007F,
    0x0000_00FF,
    0x0000_01FF,
    0x0000_03FF,
    0x0000_07FF,
    0x0000_0FFF,
    0x0000_1FFF,
    0x0000_3FFF,
    0x0000_7FFF,
    0x0000_FFFF,
    0x0001_FFFF,
    0x0003_FFFF,
    0x0007_FFFF,
    0x000F_FFFF,
    0x001F_FFFF,
    0x003F_FFFF,
    0x007F_FFFF,
    0x00FF_FFFF,
    0x01FF_FFFF,
    0x03FF_FFFF,
    0x07FF_FFFF,
    0x0FFF_FFFF,
    0x1FFF_FFFF,
    0x3FFF_FFFF,
    0x7FFF_FFFF,
    0xFFFF_FFFF,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::needless_range_loop)]
    #[test]
    fn mask() {
        for i in 0..mem::size_of::<usize>() * 8 {
            assert_eq!(ACCUM_MASK[i], (1 << i) - 1);
        }
    }
}
