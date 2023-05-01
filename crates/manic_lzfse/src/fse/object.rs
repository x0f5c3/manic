use crate::encode::{BackendType, MatchUnit};
use crate::lmd::{DMax, LMax, LmdMax, MMax};

use super::constants::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Fse;

impl LMax for Fse {
    const MAX_LITERAL_LEN: u16 = MAX_L_VALUE;
}

impl MMax for Fse {
    const MAX_MATCH_LEN: u16 = MAX_M_VALUE;
}

impl DMax for Fse {
    const MAX_MATCH_DISTANCE: u32 = MAX_D_VALUE;
}

impl LmdMax for Fse {}

impl MatchUnit for Fse {
    const MATCH_UNIT: u32 = 4;

    const MATCH_MASK: u32 = 0xFFFF_FFFF;

    #[inline(always)]
    fn match_us(us: (u32, u32)) -> u32 {
        if us.0 == us.1 {
            4
        } else {
            0
        }
    }

    #[inline(always)]
    fn hash_u(u: u32) -> u32 {
        // Donald Knuth's multiplicative hash as per LZFSE reference.
        // As we don't swap bytes, big/ little endian implementations will produce different,
        // although presumably statistically equivalent outputs.
        u.wrapping_mul(0x9E37_79B1)
    }
}

impl BackendType for Fse {}
