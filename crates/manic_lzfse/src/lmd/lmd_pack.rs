use super::lmd_type::LmdMax;
use super::lmd_type::*;

use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(align(8))]
pub struct LmdPack<T: LmdMax>(pub LiteralLenPack<T>, pub MatchLenPack<T>, pub MatchDistancePack<T>);

impl<T: LmdMax> LmdPack<T> {
    #[allow(dead_code)]
    #[inline(always)]
    pub fn new(literal_len: u16, match_len: u16, match_distance: u32) -> Self {
        Self(
            LiteralLenPack::new(literal_len),
            MatchLenPack::new(match_len),
            MatchDistancePack::new(match_distance),
        )
    }

    #[inline(always)]
    pub unsafe fn new(literal_len: u16, match_len: u16, match_distance: u32) -> Self {
        Self(
            LiteralLenPack::new(literal_len),
            MatchLenPack::new(match_len),
            MatchDistancePack::new(match_distance),
        )
    }
}

impl<T: LmdMax> Default for LmdPack<T> {
    #[inline(always)]
    fn default() -> Self {
        Self(LiteralLenPack::default(), MatchLenPack::default(), MatchDistancePack::default())
    }
}

impl<T: LmdMax> fmt::Debug for LmdPack<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("LmdPack")
            .field(&self.0.get())
            .field(&self.1.get())
            .field(&self.2.get())
            .finish()
    }
}
