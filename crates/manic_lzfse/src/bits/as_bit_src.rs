use crate::ops::Len;

use super::BitSrc;
use super::ByteBits;

pub trait AsBitSrc: Len {
    type BitSrc: BitSrc;

    /// `self.len()` minimum of 8 bytes.
    fn as_bit_src(&self) -> Self::BitSrc;
}

impl<'a> AsBitSrc for &'a [u8] {
    type BitSrc = ByteBits<'a>;

    #[inline(always)]
    fn as_bit_src(&self) -> Self::BitSrc {
        ByteBits::new(self)
    }
}
