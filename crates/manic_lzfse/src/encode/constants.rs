use crate::ring::{RingBlock, RingSize, RingType};

pub const CLAMP_INTERVAL: u32 = 0x4000_0000;

pub const GOOD_MATCH_LEN: u32 = 0x0028;

pub const RAW_CUTOFF: u32 = 0x0014;

pub const VN_CUTOFF: u32 = 0x1000;

#[derive(Copy, Clone, Debug)]
pub struct Input;

impl RingSize for Input {
    const RING_SIZE: u32 = 0x0008_0000;
}

impl RingType for Input {
    const RING_LIMIT: u32 = 0x0140;
}

impl RingBlock for Input {
    const RING_BLK_SIZE: u32 = 0x0000_4000;
}

#[derive(Copy, Clone, Debug)]
pub struct Output;

impl RingSize for Output {
    const RING_SIZE: u32 = 0x0002_0000;
}

impl RingType for Output {
    const RING_LIMIT: u32 = 0x0400;
}

impl RingBlock for Output {
    const RING_BLK_SIZE: u32 = 0x2000;
}
