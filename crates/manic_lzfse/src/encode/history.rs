use crate::types::Idx;

use super::constants::CLAMP_INTERVAL;
use super::match_unit::MatchUnit;

use std::ops::Deref;

pub const HASH_BITS: u32 = 14;

// Aligned/ power of two values. Minimum 4.
pub const HASH_WIDTH: usize = 4;

pub struct HistoryTable(Box<[History]>);

impl HistoryTable {
    const SIZE: usize = 1 << HASH_BITS;

    #[inline(always)]
    pub fn get_mut<M: MatchUnit>(&mut self, u: u32) -> &mut History {
        unsafe { self.0.get_mut(index::<M>(u)) }
    }

    #[inline(always)]
    pub fn push<M: MatchUnit>(&mut self, item: UIdx) -> History {
        let queue = self.get_mut::<M>(item.u);
        let copy = *queue;
        queue.push(item);
        copy
    }

    #[cold]
    pub fn clamp(&mut self, idx: Idx) {
        self.0.iter_mut().for_each(|u| u.clamp(idx));
    }

    #[cold]
    pub fn reset_idx(&mut self, idx: Idx) {
        self.0.iter_mut().for_each(|u| u.reset_idx(idx));
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self(vec![History::default(); Self::SIZE].into_boxed_slice())
    }
}

#[repr(align(32))]
#[derive(Copy, Clone)]
pub struct History([UIdx; HASH_WIDTH]);

impl History {
    #[allow(clippy::assertions_on_constants)]
    #[inline(always)]
    fn new(item: UIdx) -> Self {
        assert!(HASH_WIDTH >= 4);
        Self([item; HASH_WIDTH])
    }

    #[inline(always)]
    pub fn push(&mut self, item: UIdx) {
        debug_assert!(((item.idx - self.0[0].idx) as u32) < CLAMP_INTERVAL * 3);
        let mut i = HASH_WIDTH - 1;
        while i != 0 {
            self.0[i] = self.0[i - 1];
            i -= 1;
        }
        self.0[0] = item;
    }

    #[inline(always)]
    fn clamp(&mut self, idx: Idx) {
        let clamp = idx - CLAMP_INTERVAL;
        for u_idx in self.0.iter_mut().rev() {
            debug_assert!(((idx - u_idx.idx) as u32) < CLAMP_INTERVAL * 3);
            if (idx - u_idx.idx) as u32 > CLAMP_INTERVAL {
                u_idx.idx = clamp;
            } else {
                break;
            }
        }
    }

    #[inline(always)]
    fn reset_idx(&mut self, idx: Idx) {
        self.0[0].idx = idx;
    }
}

impl Default for History {
    #[inline(always)]
    fn default() -> Self {
        Self::new(UIdx::default())
    }
}

impl Deref for History {
    type Target = [UIdx];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct UIdx {
    pub u: u32,
    pub idx: Idx,
}

impl UIdx {
    #[inline(always)]
    pub fn new(u: u32, idx: Idx) -> Self {
        Self { u, idx }
    }
}

impl Default for UIdx {
    #[inline(always)]
    fn default() -> Self {
        Self { idx: Idx::default(), u: 0 }
    }
}

#[inline(always)]
fn index<M: MatchUnit>(u: u32) -> usize {
    (M::hash_u(u) >> (32 - HASH_BITS)) as usize
}
