use crate::ops::{CopyLong, CopyShort, Len, Limit, PeekData, ReadData, ShortLimit, Skip};

pub trait ShortBuffer:
    CopyShort + CopyLong + Len + Limit + PeekData + ReadData + ShortLimit + Skip
{
    fn short_bytes(&self) -> &[u8];
}

impl ShortBuffer for &[u8] {
    #[inline(always)]
    fn short_bytes(&self) -> &[u8] {
        self
    }
}

impl<T: ShortBuffer + ?Sized> ShortBuffer for &mut T {
    #[inline(always)]
    fn short_bytes(&self) -> &[u8] {
        (**self).short_bytes()
    }
}
