use crate::kit::WIDE;

use super::ring_type::RingType;

use std::marker::PhantomData;

pub struct RingBox<T>(pub(super) Box<[u8]>, PhantomData<T>);

impl<T: RingType> Default for RingBox<T> {
    fn default() -> Self {
        assert!(WIDE <= T::RING_SIZE as usize);
        assert!(T::RING_SIZE <= 0x4000_0000);
        assert!(T::RING_SIZE.is_power_of_two());
        assert!(T::RING_LIMIT <= T::RING_SIZE);
        Self(vec![0u8; T::RING_CAPACITY as usize].into_boxed_slice(), PhantomData::default())
    }
}
