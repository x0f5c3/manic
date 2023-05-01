use crate::encode::{BackendType, MatchUnit};
use crate::lmd::{DMax, LMax, LmdMax, MMax};

use super::constants::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Vn;

impl LMax for Vn {
    const MAX_LITERAL_LEN: u16 = MAX_L_VALUE;
}

impl MMax for Vn {
    const MAX_MATCH_LEN: u16 = MAX_M_VALUE;
}

impl DMax for Vn {
    const MAX_MATCH_DISTANCE: u32 = MAX_D_VALUE;
}

impl LmdMax for Vn {}

impl MatchUnit for Vn {
    const MATCH_UNIT: u32 = 3;

    #[cfg(target_endian = "little")]
    const MATCH_MASK: u32 = 0x00FF_FFFF;

    #[cfg(target_endian = "big")]
    const MATCH_MASK: u32 = 0xFFFF_FF00;

    #[inline(always)]
    fn hash_u(mut u: u32) -> u32 {
        // Donald Knuth's multiplicative hash for big/ little endian simplicity, as opposed to the
        // undefined hash function as per LZFSE reference.
        // As we don't swap bytes, big/ little endian implementations will produce different,
        // although presumably statistically equivalent outputs.
        #[cfg(target_endian = "little")]
        {
            u &= Self::MATCH_MASK;
        }
        #[cfg(target_endian = "big")]
        {
            u >>= 8;
        }
        u.wrapping_mul(0x9E37_79B1)
    }

    #[inline(always)]
    fn match_us(us: (u32, u32)) -> u32 {
        let x = us.0 ^ us.1;
        if x == 0 {
            4
        } else if x & Self::MATCH_MASK == 0 {
            3
        } else {
            0
        }
    }
}

impl BackendType for Vn {}
