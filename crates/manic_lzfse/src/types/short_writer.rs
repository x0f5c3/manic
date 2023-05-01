use crate::bits::BitDst;
use crate::ops::{Allocate, Flush, PatchInto, Pos, ShortLimit, Truncate, WriteLong, WriteShort};

pub trait ShortWriter:
    Allocate + Flush + PatchInto + Pos + BitDst + ShortLimit + Truncate + WriteShort + WriteLong
{
}

impl ShortWriter for Vec<u8> {}

impl<T: ShortWriter + ?Sized> ShortWriter for &mut T {}
