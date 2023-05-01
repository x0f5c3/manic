use crate::ops::ShortLimit;

use std::marker::PhantomData;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ShortLen<T: ?Sized>(u32, PhantomData<T>);

impl<T: ShortLimit + ?Sized> ShortLen<T> {
    #[inline(always)]
    pub fn new(u: u32) -> Self {
        assert!(u <= T::SHORT_LIMIT);
        Self(u, PhantomData::default())
    }

    #[inline(always)]
    pub unsafe fn new(u: u32) -> Self {
        debug_assert!(u <= T::SHORT_LIMIT);
        Self(u, PhantomData::default())
    }

    #[inline(always)]
    pub fn get(self) -> u32 {
        self.0
    }
}

impl<T: ShortLimit> Default for ShortLen<T> {
    #[inline(always)]
    fn default() -> Self {
        Self(0, PhantomData::default())
    }
}
