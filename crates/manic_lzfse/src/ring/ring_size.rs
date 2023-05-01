/// # Safety
///
/// * `RING_SIZE.is_power_of_two()`
/// * `WIDE <= RING_SIZE`
/// * `RING_SIZE <= 0x4000_0000`
pub trait RingSize {
    const RING_SIZE: u32;
}
