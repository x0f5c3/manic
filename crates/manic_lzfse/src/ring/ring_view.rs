use crate::bits::AsBitSrc;
use crate::kit::{CopyType, CopyTypeLong, W00, WIDE};
use crate::ops::{CopyLong, CopyShort, Len, Limit, PeekData, Pos, ReadData, ShortLimit, Skip};
use crate::types::{Idx, ShortBuffer};

use super::object::Ring;
use super::ring_type::RingType;
use super::{ring_size::RingSize, RingBits};

use std::marker::PhantomData;
use std::ptr;
use std::slice;

/// Immutable ring view.
#[derive(Copy, Clone)]
pub struct RingView<'a, T> {
    pub(super) ring_ptr: *const u8,
    pub(super) head: Idx,
    pub(super) tail: Idx,
    phantom: (PhantomData<T>, PhantomData<&'a ()>),
}

impl<'a, T: RingType> RingView<'a, T> {
    #[inline(always)]
    pub fn new(ring: &'a Ring<T>, head: Idx, tail: Idx) -> Self {
        debug_assert!((tail - head) as u32 <= T::RING_SIZE);
        Self {
            ring_ptr: ring.as_ptr(),
            head,
            tail,
            phantom: (PhantomData::default(), PhantomData::default()),
        }
    }
}

impl<'a, T> Len for RingView<'a, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        debug_assert!(self.head <= self.tail);
        (self.tail - self.head) as usize
    }
}

impl<'a, T> Pos for RingView<'a, T> {
    #[inline(always)]
    fn pos(&self) -> Idx {
        self.head
    }
}

impl<'a, T: RingSize> Skip for RingView<'a, T> {
    #[inline(always)]
    unsafe fn skip_unchecked(&mut self, len: usize) {
        debug_assert!(len <= self.len());
        self.head += len as u32;
    }
}

impl<'a, T: RingSize> Limit for RingView<'a, T> {
    #[inline(always)]
    fn limit(&mut self, len: usize) {
        let len = self.len().min(len);
        self.tail = self.head + len as u32;
    }
}

impl<'a, T: Copy + RingType> CopyShort for RingView<'a, T> {
    #[inline(always)]
    unsafe fn copy_short_raw<V: CopyType>(&self, dst: *mut u8, short_len: usize) {
        debug_assert!(short_len <= Self::SHORT_LIMIT as usize);
        debug_assert!(short_len <= self.len());
        let index = self.head % T::RING_SIZE as usize;
        debug_assert!(index + short_len <= T::RING_SIZE as usize + T::RING_LIMIT as usize);
        let src = self.ring_ptr.add(index);
        V::wide_copy::<W00>(src, dst, short_len);
    }
}

impl<'a, T: Copy + RingType> CopyLong for RingView<'a, T> {
    #[inline(always)]
    unsafe fn copy_long_raw(&self, mut dst: *mut u8, mut len: usize) {
        debug_assert!(len <= self.len());
        let mut idx = self.head;
        loop {
            let index = idx % T::RING_SIZE as usize;
            let limit = T::RING_SIZE as usize - index;
            let src = self.ring_ptr.add(index);
            if len < limit {
                debug_assert!(index + len <= T::RING_SIZE as usize + T::RING_LIMIT as usize);
                CopyTypeLong::wide_copy::<W00>(src, dst, len);
                break;
            }
            debug_assert!(index + limit <= T::RING_SIZE as usize + T::RING_LIMIT as usize);
            CopyTypeLong::wide_copy::<W00>(src, dst, limit);
            len -= limit;
            idx += limit as u32;
            dst = dst.add(limit);
        }
    }
}

impl<'a, T: RingSize> PeekData for RingView<'a, T> {
    #[inline(always)]
    unsafe fn peek_data(&self, dst: &mut [u8]) {
        debug_assert!(dst.len() <= WIDE);
        debug_assert!(self.head <= self.tail);
        let index = self.head % T::RING_SIZE as usize;
        let len = dst.len();
        let src = self.ring_ptr.add(index);
        let dst = dst.as_mut_ptr();
        ptr::copy_nonoverlapping(src, dst, len);
    }
}

impl<'a, T: RingSize> ReadData for RingView<'a, T> {
    #[inline(always)]
    unsafe fn read_data(&mut self, dst: &mut [u8]) {
        debug_assert!(dst.len() <= WIDE);
        debug_assert!(self.head <= self.tail);
        self.peek_data(dst);
        self.skip(dst.len());
    }
}

impl<'a, T: Copy + RingType> ShortLimit for RingView<'a, T> {
    const SHORT_LIMIT: u32 = T::RING_LIMIT;
}

impl<'a, T: Copy + RingType> ShortBuffer for RingView<'a, T> {
    #[inline(always)]
    fn short_bytes(&self) -> &[u8] {
        let len = self.len().min(T::RING_LIMIT as usize);
        let index = self.head % T::RING_SIZE as usize;
        let src = unsafe { self.ring_ptr.add(index) };
        unsafe { slice::from_raw_parts(src, len) }
    }
}

impl<'a, T: Copy + RingType> AsBitSrc for RingView<'a, T> {
    type BitSrc = RingBits<'a, T>;

    #[inline(always)]
    fn as_bit_src(&self) -> Self::BitSrc {
        RingBits::new(*self)
    }
}
