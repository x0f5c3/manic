use super::ring_type::RingType;
/// # Safety
///
/// * `RING_BLK_SIZE != 0`
/// * `RING_BLK_SIZE <= RING_SIZE`
pub trait RingBlock: RingType {
    const RING_BLK_SIZE: u32;
}
