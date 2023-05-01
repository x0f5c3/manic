use crate::kit::PackBits;

// SmlL - 1110LLLL
//
// * literal_len: 0x01..0x10
#[inline(always)]
pub fn encode_sml_l(literal_len: u32) -> u32 {
    debug_assert!(0x00 < literal_len);
    debug_assert!(literal_len < 0x10);
    let mut opu = 0;
    opu.set_bits(0, 4, literal_len);
    opu.set_bits(4, 4, 0xE);
    opu
}

#[allow(clippy::clippy::let_and_return)]
#[inline(always)]
pub fn decode_sml_l(opu: u32) -> u32 {
    let literal_len = opu.get_bits(0, 4);
    debug_assert_eq!(opu.get_bits(4, 4), 0xE);
    debug_assert!(0x00 < literal_len);
    debug_assert!(literal_len < 0x10);
    literal_len
}

// LrgL - 11100000 LLLLLLLL
//
// * literal_len: 0x0010..0x0110
#[inline(always)]
pub fn encode_lrg_l(literal_len: u32) -> u32 {
    debug_assert!(0x000F < literal_len);
    debug_assert!(literal_len < 0x0110);
    let mut opu = 0;
    opu.set_bits(0, 8, 0xE0);
    opu.set_bits(8, 8, literal_len - 0x10);
    opu
}

#[allow(clippy::clippy::let_and_return)]
#[inline(always)]
pub fn decode_lrg_l(opu: u32) -> u32 {
    debug_assert_eq!(opu.get_bits(0, 8), 0xE0);
    let literal_len = opu.get_bits(8, 8) + 0x10;
    debug_assert!(0x000F < literal_len);
    debug_assert!(literal_len < 0x0110);
    literal_len
}

// SmlM - 1111MMMM # previous match distance
//
// * match_len: 0x01..0x10
#[inline(always)]
pub fn encode_sml_m(match_len: u32) -> u32 {
    debug_assert!(0x00 < match_len);
    debug_assert!(match_len < 0x10);
    let mut opu = 0;
    opu.set_bits(0, 4, match_len);
    opu.set_bits(4, 4, 0xF);
    opu
}

#[allow(clippy::clippy::let_and_return)]
#[inline(always)]
pub fn decode_sml_m(opu: u32) -> u32 {
    let match_len = opu.get_bits(0, 4);
    debug_assert_eq!(opu.get_bits(4, 4), 0xF);
    debug_assert!(0x00 < match_len);
    debug_assert!(match_len < 0x10);
    match_len
}

// LrgM - 11110000 MMMMMMMM # previous match distance
//
// * match_len: 0x0010..0x0110
#[inline(always)]
pub fn encode_lrg_m(match_len: u32) -> u32 {
    debug_assert!(0x000F < match_len);
    debug_assert!(match_len >= 0x0010);
    let mut opu = 0;
    opu.set_bits(0, 8, 0xF0);
    opu.set_bits(8, 8, match_len - 0x10);
    opu
}

#[allow(clippy::clippy::let_and_return)]
#[inline(always)]
pub fn decode_lrg_m(opu: u32) -> u32 {
    debug_assert_eq!(opu.get_bits(0, 8), 0xF0);
    let match_len = opu.get_bits(8, 8) + 0x10;
    debug_assert!(0x000F < match_len);
    debug_assert!(match_len >= 0x0010);
    match_len
}

// PreD - LLMMM110 # previous match distance
//
// * literal_len: 0x01..=0x03
// * match_len:   0x03..=match_len.min(0x0A - 2 * literal_len)
#[inline(always)]
pub fn encode_pre_d(literal_len: u32, match_len: u32) -> u32 {
    debug_assert!(0x00 < literal_len);
    debug_assert!(literal_len <= 0x03);
    debug_assert!(0x02 < match_len);
    debug_assert!(match_len <= match_len_x(literal_len));
    let mut opu = 0;
    opu.set_bits(0, 3, 0x6);
    opu.set_bits(3, 3, match_len - 0x03);
    opu.set_bits(6, 2, literal_len);
    opu
}

#[allow(clippy::clippy::let_and_return)]
#[inline(always)]
pub fn decode_pre_d(opu: u32) -> (u32, u32) {
    debug_assert_eq!(opu.get_bits(0, 3), 0x6);
    let match_len = opu.get_bits(3, 3) + 0x03;
    let literal_len = opu.get_bits(6, 2);
    debug_assert!(0x00 < literal_len);
    debug_assert!(literal_len <= 0x03);
    debug_assert!(0x02 < match_len);
    debug_assert!(match_len <= match_len_x(literal_len));
    (literal_len, match_len)
}

// SmlD - LLMMMDDD DDDDDDDD
//
// * literal_len:    0x00..=0x03
// * match_len:      0x03..=match_len.min(0x0A - 2 * literal_len)
// * match_distance: 0x0000..=0x05FF
#[inline(always)]
pub fn encode_sml_d(literal_len: u32, match_len: u32, match_distance: u32) -> u32 {
    debug_assert!(literal_len <= 0x03);
    debug_assert!(0x02 < match_len);
    debug_assert!(match_len <= match_len_x(literal_len));
    debug_assert!(match_distance <= 0x05FF);
    let mut opu = 0;
    opu.set_bits(0, 3, match_distance.get_bits(8, 3));
    opu.set_bits(3, 3, match_len - 0x03);
    opu.set_bits(6, 2, literal_len);
    opu.set_bits(8, 8, match_distance.get_bits(0, 8));
    opu
}

#[allow(clippy::clippy::let_and_return)]
#[inline(always)]
pub fn decode_sml_d(opu: u32) -> (u32, u32, u32) {
    let mut match_distance = 0;
    match_distance.set_bits(8, 3, opu.get_bits(0, 3));
    let match_len = opu.get_bits(3, 3) + 0x03;
    let literal_len = opu.get_bits(6, 2);
    match_distance.set_bits(0, 8, opu.get_bits(8, 8));
    debug_assert!(literal_len <= 0x03);
    debug_assert!(0x02 < match_len);
    debug_assert!(match_len <= match_len_x(literal_len));
    debug_assert!(match_distance <= 0x05FF);
    (literal_len, match_len, match_distance)
}

// MedD - 101LLMMM DDDDDDMM DDDDDDDD
//
// * literal_len:    0x00..=0x03
// * match_len:      0x03..=0x22
// * match_distance: 0x0000..=0x3FFF
#[inline(always)]
pub fn encode_med_d(literal_len: u32, match_len: u32, match_distance: u32) -> u32 {
    debug_assert!(literal_len <= 0x03);
    debug_assert!(0x02 < match_len);
    debug_assert!(match_len <= 0x22);
    debug_assert!(match_distance <= 0x3FFF);
    let match_len = match_len - 0x03;
    let mut opu = 0;
    opu.set_bits(0, 3, (match_len as u32).get_bits(2, 3));
    opu.set_bits(3, 2, literal_len as u32);
    opu.set_bits(5, 3, 0x5);
    opu.set_bits(8, 2, (match_len as u32).get_bits(0, 2));
    opu.set_bits(10, 14, match_distance as u32);
    opu
}

#[allow(clippy::clippy::let_and_return)]
#[inline(always)]
pub fn decode_med_d(opu: u32) -> (u32, u32, u32) {
    let mut match_len = 0;
    match_len.set_bits(2, 3, opu.get_bits(0, 3));
    let literal_len = opu.get_bits(3, 2);
    debug_assert_eq!(opu.get_bits(5, 3), 0x5);
    match_len.set_bits(0, 2, opu.get_bits(8, 2));
    let match_distance = opu.get_bits(10, 14);
    match_len += 0x03;
    debug_assert!(literal_len <= 0x03);
    debug_assert!(0x02 < match_len);
    debug_assert!(match_len <= 0x22);
    debug_assert!(match_distance <= 0x3FFF);
    (literal_len, match_len, match_distance)
}

// LrgD - LLMMM111 DDDDDDDD DDDDDDDD
//
// * literal_len:    0x00..=0x03
// * match_len:      0x03..=match_len.min(0x0A - 2 * literal_len)
// * match_distance: 0x0000..=0xFFFF
#[inline(always)]
pub fn encode_lrg_d(literal_len: u32, match_len: u32, match_distance: u32) -> u32 {
    debug_assert!(literal_len <= 0x03);
    debug_assert!(0x02 < match_len);
    debug_assert!(match_len <= match_len_x(literal_len));
    debug_assert!(match_distance <= 0xFFFF);
    let mut opu = 0;
    opu.set_bits(0, 3, 0x7);
    opu.set_bits(3, 3, match_len - 0x03);
    opu.set_bits(6, 2, literal_len);
    opu.set_bits(8, 16, match_distance);
    opu
}

#[allow(clippy::clippy::let_and_return)]
#[inline(always)]
pub fn decode_lrg_d(opu: u32) -> (u32, u32, u32) {
    debug_assert_eq!(opu.get_bits(0, 3), 0x7);
    let match_len = opu.get_bits(3, 3) + 0x03;
    let literal_len = opu.get_bits(6, 2);
    let match_distance = opu.get_bits(8, 16);
    debug_assert!(literal_len <= 0x03);
    debug_assert!(0x02 < match_len);
    debug_assert!(match_len <= match_len_x(literal_len));
    debug_assert!(match_distance <= 0xFFFF);
    (literal_len, match_len, match_distance)
}

#[inline(always)]
pub fn match_len_x(literal_len: u32) -> u32 {
    debug_assert!(literal_len <= 0x03);
    0x0A - 0x02 * literal_len
}

#[cfg(test)]
mod tests {
    use crate::vn::constants::*;

    use super::*;

    #[test]
    #[ignore = "expensive"]
    fn sml_l_encode_decode() {
        for literal_len in 0x01..0x10 {
            let opu = encode_sml_l(literal_len);
            assert_eq!(OP_TABLE[opu as usize & 0xFF], Op::SmlL);
            assert_eq!(decode_sml_l(opu), literal_len);
        }
    }

    #[test]
    #[ignore = "expensive"]
    fn lrg_l_encode_decode() {
        for literal_len in 0x0010..0x0110 {
            let opu = encode_lrg_l(literal_len);
            assert_eq!(OP_TABLE[opu as usize & 0xFF], Op::LrgL);
            assert_eq!(decode_lrg_l(opu), literal_len);
        }
    }

    #[test]
    #[ignore = "expensive"]
    fn sml_m_encode_decode() {
        for match_len in 0x01..0x10 {
            let opu = encode_sml_m(match_len);
            assert_eq!(OP_TABLE[opu as usize & 0xFF], Op::SmlM);
            assert_eq!(decode_sml_m(opu), match_len);
        }
    }

    #[test]
    #[ignore = "expensive"]
    fn lrg_m_encode_decode() {
        for match_len in 0x0010..0x0110 {
            let opu = encode_lrg_m(match_len);
            assert_eq!(OP_TABLE[opu as usize & 0xFF], Op::LrgM);
            assert_eq!(decode_lrg_m(opu), match_len);
        }
    }

    #[test]
    #[ignore = "expensive"]
    fn pre_d_encode_decode() {
        for literal_len in 0x01..0x04 {
            for match_len in 0x03..=match_len_x(literal_len) {
                let opu = encode_pre_d(literal_len, match_len);
                assert_eq!(OP_TABLE[opu as usize & 0xFF], Op::PreD);
                assert_eq!(decode_pre_d(opu), (literal_len, match_len));
            }
        }
    }

    #[test]
    #[ignore = "expensive"]
    fn sml_d_encode_decode() {
        for literal_len in 0x00..0x04 {
            for match_len in 0x03..=match_len_x(literal_len) {
                for match_distance in 0x0000..0x0600 {
                    let opu = encode_sml_d(literal_len, match_len, match_distance);
                    assert_eq!(OP_TABLE[opu as usize & 0xFF], Op::SmlD);
                    assert_eq!(decode_sml_d(opu), (literal_len, match_len, match_distance));
                }
            }
        }
    }

    #[test]
    #[ignore = "expensive"]
    fn med_d_encode_decode() {
        for literal_len in 0x00..0x04 {
            for match_len in 0x03..0x23 {
                for match_distance in 0x0000..0x4000 {
                    let opu = encode_med_d(literal_len, match_len, match_distance);
                    assert_eq!(OP_TABLE[opu as usize & 0xFF], Op::MedD);
                    assert_eq!(decode_med_d(opu), (literal_len, match_len, match_distance));
                }
            }
        }
    }

    #[test]
    #[ignore = "expensive"]
    fn lrg_d_encode_decode() {
        for literal_len in 0x01..0x04 {
            for match_len in 0x03..=match_len_x(literal_len) {
                for match_distance in 0x0000..0x1_0000 {
                    let opu = encode_lrg_d(literal_len, match_len, match_distance);
                    assert_eq!(OP_TABLE[opu as usize & 0xFF], Op::LrgD);
                    assert_eq!(decode_lrg_d(opu), (literal_len, match_len, match_distance));
                }
            }
        }
    }

    // Opcodes are defined by a maximum of 3 bytes. Here we decode all combinations, 0 - 0x00FF_FFF,
    // with an additional margin. We want to test that decodes are working as intended and that
    // all literal len, match len and match distance values are within bounds.
    #[test]
    #[ignore = "expensive"]
    fn decode_encode() {
        for opu in 0..0x0800_0000 {
            match OP_TABLE[opu as usize & 0xFF] {
                Op::SmlL => {
                    let literal_len = decode_sml_l(opu);
                    assert_eq!(encode_sml_l(literal_len), opu & 0x0000_00FF);
                }
                Op::LrgL => {
                    let literal_len = decode_lrg_l(opu);
                    assert_eq!(encode_lrg_l(literal_len), opu & 0x0000_FFFF);
                }
                Op::SmlM => {
                    let match_len = decode_sml_m(opu);
                    assert_eq!(encode_sml_m(match_len), opu & 0x0000_00FF);
                }
                Op::LrgM => {
                    let match_len = decode_lrg_m(opu);
                    assert_eq!(encode_lrg_m(match_len), opu & 0x0000_FFFF);
                }
                Op::PreD => {
                    let (literal_len, match_len) = decode_pre_d(opu);
                    assert_eq!(encode_pre_d(literal_len, match_len), opu & 0x0000_00FF);
                }
                Op::SmlD => {
                    let (literal_len, match_len, match_distance) = decode_sml_d(opu);
                    assert_eq!(
                        encode_sml_d(literal_len, match_len, match_distance),
                        opu & 0x0000_FFFF
                    );
                }
                Op::MedD => {
                    let (literal_len, match_len, match_distance) = decode_med_d(opu);
                    assert_eq!(
                        encode_med_d(literal_len, match_len, match_distance),
                        opu & 0x00FF_FFFF
                    );
                }
                Op::LrgD => {
                    let (literal_len, match_len, match_distance) = decode_lrg_d(opu);
                    assert_eq!(
                        encode_lrg_d(literal_len, match_len, match_distance),
                        opu & 0x00FF_FFFF
                    );
                }
                Op::Eos | Op::Nop | Op::Udef => {}
            };
        }
    }
}
