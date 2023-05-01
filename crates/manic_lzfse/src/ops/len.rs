pub trait Len {
    fn len(&self) -> usize;

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Len for [u8] {
    #[inline(always)]
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }
}

impl<T: Len + ?Sized> Len for &T {
    #[inline(always)]
    fn len(&self) -> usize {
        (**self).len()
    }
}

impl<T: Len + ?Sized> Len for &mut T {
    #[inline(always)]
    fn len(&self) -> usize {
        (**self).len()
    }
}
