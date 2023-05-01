use super::lmd_type::LmdMax;
use super::lmd_type::*;

use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Lmd<T: LmdMax>(pub LiteralLen<T>, pub MatchLen<T>, pub MatchDistance<T>);

impl<T: LmdMax> Lmd<T> {
    #[allow(dead_code)]
    pub fn new(literal_len: u32, match_len: u32, match_distance: u32) -> Self {
        Self(
            LiteralLen::new(literal_len),
            MatchLen::new(match_len),
            MatchDistance::new(match_distance),
        )
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub unsafe fn new(literal_len: u32, match_len: u32, match_distance: u32) -> Self {
        Self(
            LiteralLen::new(literal_len),
            MatchLen::new(match_len),
            MatchDistance::new(match_distance),
        )
    }
}

impl<T: LmdMax> Default for Lmd<T> {
    #[inline(always)]
    fn default() -> Self {
        Self(LiteralLen::default(), MatchLen::default(), MatchDistance::default())
    }
}

impl<T: LmdMax> fmt::Debug for Lmd<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("Lmd")
            .field(&self.0.get())
            .field(&self.1.get())
            .field(&self.2.get())
            .finish()
    }
}
