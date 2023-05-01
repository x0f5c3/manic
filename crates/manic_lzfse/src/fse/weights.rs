use crate::lmd::LmdPack;
use crate::types::ShortWriter;

use super::constants::{self, *};
use super::error_kind::FseErrorKind;
use super::weight_encoder::{self};
use super::Fse;
use std::mem;

use std::io;

/// Normalized L, M, D and U weight tables. Individual table totals are guaranteed to lie within
/// their respective state boundaries (0..=N_STATES).
#[derive(Debug)]
pub struct Weights([u16; N_WEIGHTS]);

// Implementation notes:
//
// Tables are defined as a single compound array to simplify V2 compression: [ L | M | D | U ]
//
// Tables are maintained in a normalized state.

impl Weights {
    #[inline(never)]
    pub fn load(&mut self, lmds: &[LmdPack<Fse>], literals: &[u8]) -> u8 {
        self.reset_weights();
        self.add_lmds(lmds);
        self.add_literals(literals)
    }

    pub fn reset_weights(&mut self) {
        reset_weights(&mut self.0);
    }

    fn add_lmds(&mut self, lmds: &[LmdPack<Fse>]) {
        if lmds.is_empty() {
            return;
        }
        for LmdPack(literal_len, match_len, match_distance_zeroed) in lmds.iter() {
            // We trust the LmdPack<Fse> structure and omit bounds checking.
            let d_index = constants::d_index(match_distance_zeroed.get() as usize);
            unsafe {
                let l_base = *L_BASE_FROM_VALUE.get(literal_len.get() as usize) as usize;
                let m_base = *M_BASE_FROM_VALUE.get(match_len.get() as usize) as usize;
                let d_base = *D_BASE_FROM_VALUE.get(d_index) as usize;
                *self.0[L_RANGE].get_mut(l_base) += 1;
                *self.0[M_RANGE].get_mut(m_base) += 1;
                *self.0[D_RANGE].get_mut(d_base) += 1
            }
        }
        normalize_m1(&mut self.0[L_RANGE], lmds.len() as u32, L_STATES);
        normalize_m1(&mut self.0[M_RANGE], lmds.len() as u32, M_STATES);
        normalize_m1(&mut self.0[D_RANGE], lmds.len() as u32, D_STATES);
    }

    fn add_literals(&mut self, literals: &[u8]) -> u8 {
        if literals.is_empty() {
            return 0;
        }
        for &u in literals {
            (&mut self.0[U_RANGE])[u as usize] += 1;
        }
        normalize_m1(&mut self.0[U_RANGE], literals.len() as u32, U_STATES) as u8
    }

    pub fn load_v1(&mut self, src: &[u8]) -> crate::Result<()> {
        if src.len() < V1_WEIGHT_PAYLOAD_BYTES as usize {
            return Err(FseErrorKind::WeightPayloadUnderflow.into());
        }
        if src.len() > V1_WEIGHT_PAYLOAD_BYTES as usize {
            return Err(FseErrorKind::WeightPayloadOverflow.into());
        }
        let src = src.as_ptr();
        let dst = self.0.as_mut_ptr();
        for off in 0..N_WEIGHTS {
            let w = (unsafe { src.cast::<u16>().add(off).read_unaligned() }).to_le();
            unsafe { *dst.add(off) = w };
        }
        self.check_totals()
    }

    #[allow(arithmetic_overflow)]
    pub fn load_v2(&mut self, src: &[u8]) -> crate::Result<()> {
        let mut accum: usize = 0;
        let mut accum_bits: isize = 0;
        let mut i = 0;
        for weight in self.0.iter_mut() {
            while i != src.len() && accum_bits <= 24 {
                accum |= (unsafe { *src.get(i) } as usize) << accum_bits;
                accum_bits += 8;
                i += 1;
            }
            let (w, w_bits) = weight_encoder::decode_weight(accum);
            *weight = w as u16;
            accum >>= w_bits;
            accum_bits -= w_bits as isize;
        }
        if accum_bits < 0 {
            return Err(FseErrorKind::WeightPayloadUnderflow.into());
        }
        if accum_bits >= 8 || i != src.len() {
            return Err(FseErrorKind::WeightPayloadOverflow.into());
        }
        self.check_totals()
    }

    pub fn store_v1_short<O: ShortWriter>(&self, dst: &mut O) -> io::Result<()> {
        let mut wide_bytes = dst.short_block(V1_WEIGHT_PAYLOAD_BYTES)?;
        self.store_v1(&mut wide_bytes);
        Ok(())
    }

    #[allow(clippy::needless_range_loop)]
    pub fn store_v1(&self, dst: &mut [u8]) {
        assert!(N_WEIGHTS * mem::size_of::<u16>() <= V1_WEIGHT_PAYLOAD_BYTES as usize);
        assert!(dst.len() >= V1_WEIGHT_PAYLOAD_BYTES as usize);
        for i in 0..N_WEIGHTS {
            let w = self.0[i];
            let bytes = w.to_le_bytes();
            let j = i * 2;
            unsafe { dst.get_mut(j..j + mem::size_of::<u16>()).copy_from_slice(&bytes) };
        }
        for i in N_WEIGHTS * 2..V1_WEIGHT_PAYLOAD_BYTES as usize {
            dst[i] = 0;
        }
    }

    pub fn store_v2_short<O: ShortWriter>(&self, dst: &mut O) -> io::Result<u32> {
        let pos = dst.pos();
        let mut wide_bytes = dst.short_block(V2_WEIGHT_PAYLOAD_BYTES_MAX)?;
        let n = self.store_v2(&mut wide_bytes);
        dst.truncate(pos + n);
        Ok(n)
    }

    #[allow(clippy::assertions_on_constants)]
    pub fn store_v2(&self, dst: &mut [u8]) -> u32 {
        assert!(dst.len() >= V2_WEIGHT_PAYLOAD_BYTES_MAX as usize);
        debug_assert_eq!(self.0.len(), N_WEIGHTS);
        let mut accum: usize = 0;
        let mut accum_bits: usize = 0;
        let mut i = 0;
        for weight in self.0.iter() {
            let (u, u_bits) = weight_encoder::encode_weight(*weight as usize);
            accum |= u << accum_bits;
            accum_bits += u_bits;
            while accum_bits >= 8 {
                debug_assert!(i <= dst.len());
                *{ dst.get_mut(i) } = accum as u8;
                accum >>= 8;
                accum_bits -= 8;
                i += 1;
            }
        }
        if accum_bits > 0 {
            debug_assert!(i <= dst.len());
            *{ dst.get_mut(i) } = accum as u8;
            i += 1;
        }
        i as u32
    }

    #[inline(always)]
    pub fn ls(&self) -> &[u16] {
        debug_assert!(total_weights(&self.0[L_RANGE]) <= L_STATES);
        &self.0[L_RANGE]
    }

    #[inline(always)]
    pub fn ms(&self) -> &[u16] {
        debug_assert!(total_weights(&self.0[M_RANGE]) <= M_STATES);
        &self.0[M_RANGE]
    }

    #[inline(always)]
    pub fn ds(&self) -> &[u16] {
        debug_assert!(total_weights(&self.0[D_RANGE]) <= D_STATES);
        &self.0[D_RANGE]
    }

    #[inline(always)]
    pub fn us(&self) -> &[u16] {
        debug_assert!(total_weights(&self.0[U_RANGE]) <= U_STATES);
        &self.0[U_RANGE]
    }

    fn check_totals(&mut self) -> crate::Result<()> {
        if total_weights(&self.0[L_RANGE]) <= L_STATES
            && total_weights(&self.0[M_RANGE]) <= M_STATES
            && total_weights(&self.0[D_RANGE]) <= D_STATES
            && total_weights(&self.0[U_RANGE]) <= U_STATES
        {
            Ok(())
        } else {
            self.0.fill(0);
            Err(FseErrorKind::BadWeightPayload.into())
        }
    }
}

impl Default for Weights {
    #[inline(always)]
    fn default() -> Self {
        Self([0; N_WEIGHTS])
    }
}

fn total_weights(weights: &[u16]) -> u32 {
    weights.iter().map(|&u| u as u32).sum::<u32>()
}

fn reset_weights(weights: &mut [u16]) {
    weights.iter_mut().for_each(|u| *u = 0);
}

pub fn normalize_m1(weights: &mut [u16], in_total: u32, out_total: u32) -> usize {
    assert!(out_total.is_power_of_two());
    assert!(out_total <= 0x4000_0000);
    assert!(weights.len() <= out_total as usize);
    debug_assert_eq!(total_weights(weights), in_total);
    let (remaining, max_weight_index) = normalize_m1_coarse(weights, in_total, out_total);
    if -remaining < weights[max_weight_index] as i32 / 4 {
        weights[max_weight_index] = (weights[max_weight_index] as i32 + remaining) as u16;
    } else {
        normalize_m1_trim(weights, -remaining as u32);
    }
    max_weight_index
}

fn normalize_m1_coarse(weights: &mut [u16], in_total: u32, out_total: u32) -> (i32, usize) {
    debug_assert!(out_total.is_power_of_two());
    debug_assert!(out_total <= 0x4000_0000);
    debug_assert!(weights.len() <= out_total as usize);
    if in_total == 0 {
        return (0, 0);
    }
    let shift = out_total.leading_zeros();
    let multiply = (1 << 31) / in_total;
    let round = 1 << (shift - 1);
    let mut max_weight = 0;
    let mut max_weight_index = 0;
    let mut remaining = out_total as i32;
    for (i, w) in weights.iter_mut().enumerate() {
        if *w == 0 {
            continue;
        }
        let mut f = (*w as u32 * multiply + round) >> shift;
        if f == 0 {
            f = 1;
        }
        *w = f as u16;
        remaining -= f as i32;
        if f > max_weight {
            max_weight = f;
            max_weight_index = i;
        }
    }
    (remaining, max_weight_index)
}

fn normalize_m1_trim(weights: &mut [u16], mut overflow: u32) {
    for shift in (0..=3).rev() {
        for w in weights.iter_mut() {
            if overflow == 0 {
                break;
            }
            if *w == 0 {
                continue;
            }
            let n = ((*w as u32 - 1) >> shift).min(overflow);
            *w -= n as u16;
            overflow -= n;
        }
    }
    assert!(overflow == 0);
}

#[cfg(test)]
mod tests {
    use test_kit::Rng;

    use super::*;

    #[test]
    fn v1_l_overflow() {
        let mut weights = Weights::default();
        (&mut weights.0[L_RANGE])[0] = L_STATES as u16;
        (&mut weights.0[L_RANGE])[1] = 1;
        let mut bs = [0u8; V1_WEIGHT_PAYLOAD_BYTES as usize];
        weights.store_v1(bs.as_mut());
        assert!(weights.load_v1(bs.as_ref()).is_err());
    }

    #[test]
    fn v1_m_overflow() {
        let mut weights = Weights::default();
        (&mut weights.0[M_RANGE])[0] = M_STATES as u16;
        (&mut weights.0[M_RANGE])[1] = 1;
        let mut bs = [0u8; V1_WEIGHT_PAYLOAD_BYTES as usize];
        weights.store_v1(bs.as_mut());
        assert!(weights.load_v1(bs.as_ref()).is_err());
    }

    #[test]
    fn v1_d_overflow() {
        let mut weights = Weights::default();
        (&mut weights.0[D_RANGE])[0] = D_STATES as u16;
        (&mut weights.0[D_RANGE])[1] = 1;
        let mut bs = [0u8; V1_WEIGHT_PAYLOAD_BYTES as usize];
        weights.store_v1(bs.as_mut());
        assert!(weights.load_v1(bs.as_ref()).is_err());
    }

    #[test]
    fn v1_u_overflow() {
        let mut weights = Weights::default();
        (&mut weights.0[U_RANGE])[0] = U_STATES as u16;
        (&mut weights.0[U_RANGE])[1] = 1;
        let mut bs = [0u8; V1_WEIGHT_PAYLOAD_BYTES as usize];
        weights.store_v1(bs.as_mut());
        assert!(weights.load_v1(bs.as_ref()).is_err());
    }

    #[test]
    fn v2_l_overflow() {
        let mut weights = Weights::default();
        (&mut weights.0[L_RANGE])[0] = L_STATES as u16;
        (&mut weights.0[L_RANGE])[1] = 1;
        let mut bs = [0u8; V2_WEIGHT_PAYLOAD_BYTES_MAX as usize];
        let n = weights.store_v2(bs.as_mut());
        assert!(weights.load_v2(&bs[..n as usize]).is_err());
    }

    #[test]
    fn v2_m_overflow() {
        let mut weights = Weights::default();
        (&mut weights.0[M_RANGE])[0] = M_STATES as u16;
        (&mut weights.0[M_RANGE])[1] = 1;
        let mut bs = [0u8; V2_WEIGHT_PAYLOAD_BYTES_MAX as usize];
        let n = weights.store_v2(bs.as_mut());
        assert!(weights.load_v2(&bs[..n as usize]).is_err());
    }

    #[test]
    fn v2_d_overflow() {
        let mut weights = Weights::default();
        (&mut weights.0[D_RANGE])[0] = D_STATES as u16;
        (&mut weights.0[D_RANGE])[1] = 1;
        let mut bs = [0u8; V2_WEIGHT_PAYLOAD_BYTES_MAX as usize];
        let n = weights.store_v2(bs.as_mut());
        assert!(weights.load_v2(&bs[..n as usize]).is_err());
    }

    #[test]
    fn v2_u_overflow() {
        let mut weights = Weights::default();
        (&mut weights.0[U_RANGE])[0] = U_STATES as u16;
        (&mut weights.0[U_RANGE])[1] = 1;
        let mut bs = [0u8; V2_WEIGHT_PAYLOAD_BYTES_MAX as usize];
        let n = weights.store_v2(bs.as_mut());
        assert!(weights.load_v2(&bs[..n as usize]).is_err());
    }

    fn normalize_check(mut weights: [u16; 12]) {
        for &out_total in &[64, 256, 1024] {
            while weights[0] != 0 {
                let mut copy = weights;
                let in_total = total_weights(&copy);
                normalize_m1(&mut copy, in_total, out_total);
                assert_eq!(total_weights(&copy), out_total);
                for (&w, &c) in weights.iter().zip(copy.iter()) {
                    if w != 0 {
                        assert!(c != 0);
                    }
                }
                for w in weights.iter_mut() {
                    if *w != 0 {
                        *w -= 1;
                    }
                }
            }
        }
    }

    fn trim_check(mut weights: [u16; 12]) {
        for &out_total in &[64, 256, 1024] {
            loop {
                let mut copy = weights;
                let in_total = total_weights(&copy);
                if in_total < out_total {
                    break;
                }
                let overflow = in_total - out_total;
                normalize_m1_trim(&mut copy, overflow);
                assert_eq!(total_weights(&copy), out_total);
                for (&w, &c) in weights.iter().zip(copy.iter()) {
                    if w != 0 {
                        assert!(c != 0);
                    }
                }
                for w in weights.iter_mut() {
                    if *w != 0 {
                        *w -= 1;
                    }
                }
            }
        }
    }

    #[test]
    fn normalize_0() {
        normalize_check([2048, 1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1]);
    }

    #[test]
    fn normalize_1() {
        normalize_check([512, 511, 510, 509, 508, 507, 506, 505, 504, 502, 501, 500]);
    }

    #[test]
    fn normalize_2() {
        normalize_check([65535, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn normalize_3() {
        normalize_check([1024; 12]);
    }

    #[test]
    #[ignore = "expensive"]
    fn normalize_rng() {
        let mut rng = Rng::default();
        for _ in 0..16384 {
            let mut weights = [0u16; 12];
            for w in weights.iter_mut() {
                let v = rng.gen() % 0x1_0000;
                let v = (v * v) / 0x10_0000;
                *w = v as u16;
            }
            normalize_check(weights);
        }
    }

    #[test]
    fn normalize_floor() {
        let mut weights = [65535, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        for &out_total in &[64, 256, 1024] {
            while weights[0] != 0 {
                let mut copy = weights;
                let in_total = total_weights(&copy);
                normalize_m1(&mut copy, in_total, out_total);
                assert_eq!(total_weights(&copy), out_total);
                for (&w, &c) in weights.iter().zip(copy.iter()) {
                    if w != 0 {
                        assert!(c != 0);
                    }
                }
                if weights[0] != 0 {
                    weights[0] -= 1;
                }
            }
        }
    }

    #[test]
    fn trim_0() {
        trim_check([2048, 1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1]);
    }

    #[test]
    fn trim_1() {
        trim_check([512, 511, 510, 509, 508, 507, 506, 505, 504, 502, 501, 500]);
    }

    #[test]
    fn trim_2() {
        trim_check([65535, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn trim_3() {
        trim_check([1024; 12]);
    }

    #[test]
    #[ignore = "expensive"]
    fn trim_rng() {
        let mut rng = Rng::default();
        for _ in 0..0x1000 {
            let mut weights = [0u16; 12];
            for w in weights.iter_mut() {
                let v = rng.gen() % 0x1_0000;
                let v = (v * v) / 0x10_0000;
                *w = v as u16;
            }
            trim_check(weights);
        }
    }
}
