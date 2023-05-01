use std::cmp::Ordering;
use std::fmt::{Display, Formatter, LowerHex, Result, UpperHex};
use std::ops::{Add, AddAssign, Div, DivAssign, Neg, Rem, Sub, SubAssign};

/// Wrapping index type.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Idx(u32);

impl Idx {
    #[inline(always)]
    pub fn new(u: u32) -> Self {
        Self(u)
    }

    #[inline(always)]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl Default for Idx {
    #[inline(always)]
    fn default() -> Self {
        Self(0)
    }
}

impl From<Idx> for u32 {
    #[inline(always)]
    fn from(v: Idx) -> Self {
        v.0
    }
}

impl From<Idx> for usize {
    #[inline(always)]
    fn from(v: Idx) -> Self {
        v.0 as usize
    }
}

impl From<Idx> for isize {
    #[inline(always)]
    fn from(v: Idx) -> Self {
        v.0 as isize
    }
}

impl From<u32> for Idx {
    #[inline(always)]
    fn from(v: u32) -> Self {
        Self::new(v)
    }
}

impl From<u64> for Idx {
    #[inline(always)]
    fn from(v: u64) -> Self {
        Self::new(v as u32)
    }
}

impl PartialOrd for Idx {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Idx {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0 as i32).wrapping_sub(other.0 as i32).cmp(&0)
    }
}

impl Rem<u32> for Idx {
    type Output = u32;

    #[inline(always)]
    fn rem(self, rhs: u32) -> u32 {
        self.0 % rhs
    }
}

impl Rem<usize> for Idx {
    type Output = usize;

    #[inline(always)]
    fn rem(self, rhs: usize) -> usize {
        (self.0 as usize) % rhs
    }
}

impl Sub for Idx {
    type Output = i32;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        (self.0 as i32).wrapping_sub(rhs.0 as i32)
    }
}

impl Add<i32> for Idx {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: i32) -> Self::Output {
        Self(self.0.wrapping_add(rhs as u32))
    }
}

impl AddAssign<i32> for Idx {
    #[inline(always)]
    fn add_assign(&mut self, rhs: i32) {
        *self = *self + rhs
    }
}

impl Sub<i32> for Idx {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: i32) -> Self::Output {
        Self(self.0.wrapping_sub(rhs as u32))
    }
}

impl SubAssign<i32> for Idx {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: i32) {
        *self = *self - rhs
    }
}

impl Add<u32> for Idx {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl AddAssign<u32> for Idx {
    #[inline(always)]
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs
    }
}

impl Sub<u32> for Idx {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_sub(rhs))
    }
}

impl SubAssign<u32> for Idx {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self - rhs
    }
}

impl Div<u32> for Idx {
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: u32) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl DivAssign<u32> for Idx {
    #[inline(always)]
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs
    }
}

impl Neg for Idx {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        Self(self.0.wrapping_sub(self.0))
    }
}

impl Display for Idx {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Display::fmt(&self.0, f)
    }
}

impl LowerHex for Idx {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl UpperHex for Idx {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        UpperHex::fmt(&self.0, f)
    }
}
