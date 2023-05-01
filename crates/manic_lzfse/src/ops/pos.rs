use crate::types::Idx;

pub trait Pos {
    fn pos(&self) -> Idx;
}

impl Pos for Vec<u8> {
    #[inline(always)]
    fn pos(&self) -> Idx {
        (self.len() as u32).into()
    }
}

impl<T: Pos + ?Sized> Pos for &T {
    #[inline(always)]
    fn pos(&self) -> Idx {
        (**self).pos()
    }
}

impl<T: Pos + ?Sized> Pos for &mut T {
    #[inline(always)]
    fn pos(&self) -> Idx {
        (**self).pos()
    }
}
