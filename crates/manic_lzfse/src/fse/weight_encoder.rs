use super::constants::*;

#[allow(dead_code)]
#[inline(always)]
pub const fn weight_payload_limit(n_weights: usize) -> usize {
    (n_weights * MAX_W_BITS + 7) / 8
}

#[inline(always)]
pub fn decode_weight(u: usize) -> (usize, usize) {
    let index = u as usize & 0x1F;
    let u_bits = WEIGHTS_BITS_TABLE[index] as usize;
    let w = match u_bits {
        8 => 8 + ((u >> 4) & 0xF),
        14 => 24 + ((u >> 4) & 0x3FF),
        _ => WEIGHTS_VALUE_TABLE[index] as usize,
    };
    debug_assert!(w < 1048);
    (w, u_bits)
}

#[inline(always)]
pub fn encode_weight(w: usize) -> (usize, usize) {
    debug_assert!(w < 1048);
    match w {
        0 => (0, 2),
        1 => (2, 2),
        2 => (1, 3),
        3 => (5, 3),
        4 => (3, 5),
        5 => (11, 5),
        6 => (19, 5),
        7 => (27, 5),
        v if v < 24 => (((v - 8) << 4) + 7, 8),
        v => (((v - 24) << 4) + 15, 14),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_all() {
        for value in 0..1048 {
            let (u, u_bits) = encode_weight(value);
            let (v, v_bits) = decode_weight(u);
            assert_eq!(v, value);
            assert_eq!(u_bits, v_bits);
        }
    }
}
