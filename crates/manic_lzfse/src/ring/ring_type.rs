use crate::kit::WIDE;

use super::ring_size::RingSize;

/// # Safety
///
/// * `RING_LIMIT <= RING_SIZE / 2`
/// * `RING_LIMIT <= i32::MAX`
pub trait RingType: RingSize {
    const RING_LIMIT: u32;

    const RING_CAPACITY: u32 = Self::RING_SIZE + Self::RING_LIMIT * 2 + WIDE as u32;
}
