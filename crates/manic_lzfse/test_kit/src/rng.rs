/// Deterministic random number generator.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    #[inline(always)]
    pub fn gen(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.0
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Iterator for Rng {
    type Item = u32;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.gen())
    }
}
