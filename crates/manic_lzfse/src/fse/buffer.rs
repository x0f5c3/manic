use crate::bits::BitDst;
use crate::lmd::{LMax, LmdPack, MMax, MatchDistance};
use crate::ops::WriteShort;
use crate::types::ShortBuffer;

use super::block::FseBlock;
use super::constants::*;
use super::encoder::Encoder;
use super::literals::Literals;
use super::lmds::Lmds;
use super::weights::Weights;
use super::Fse;

use std::convert::AsRef;
use std::io;

pub struct Buffer {
    literals: Literals,
    lmds: Lmds,
    n_match_bytes: u32,
    match_distance: u32,
}

impl Buffer {
    pub fn pad(&mut self) {
        self.literals.pad();
    }

    pub fn init_weights(&self, weights: &mut Weights) -> u8 {
        weights.load(self.lmds.as_ref(), self.literals.as_ref())
    }

    pub fn store<O>(&self, dst: &mut O, encoder: &Encoder) -> io::Result<FseBlock>
    where
        O: BitDst + WriteShort,
    {
        let literal_param = self.literals.store(dst, encoder)?;
        let lmd_param = self.lmds.store(dst, encoder)?;
        Ok(FseBlock::new(self.n_raw_bytes(), literal_param, lmd_param).expect("internal error"))
    }

    // TODO consider simplifying.
    #[inline(always)]
    pub fn push<I>(
        &mut self,
        literals: &mut I,
        match_len: &mut u32,
        match_distance: MatchDistance<Fse>,
    ) -> bool
    where
        I: ShortBuffer,
    {
        let match_distance = match_distance.get();
        debug_assert!(literals.len() != 0 || *match_len != 0);
        while literals.len() > Fse::MAX_LITERAL_LEN as usize {
            if self.lmds.len() == LMDS_PER_BLOCK as usize {
                return false;
            }
            let limit = LITERALS_PER_BLOCK - self.literals.len() as u32;
            if Fse::MAX_LITERAL_LEN as u32 <= limit {
                unsafe { self.literals.push_unchecked_max(literals) };
                unsafe { self.push_l(Fse::MAX_LITERAL_LEN) };
            } else if limit != 0 {
                unsafe { self.literals.push_unchecked(literals, limit) };
                unsafe { self.push_l(limit as u16) };
                return false;
            } else {
                return false;
            }
        }
        if self.lmds.len() == LMDS_PER_BLOCK as usize {
            return false;
        }
        let mut literal_len = literals.len();
        let limit = LITERALS_PER_BLOCK - self.literals.len() as u32;
        if literal_len <= limit as usize {
            unsafe { self.literals.push_unchecked(literals, literal_len as u32) };
        } else if limit != 0 {
            unsafe { self.literals.push_unchecked(literals, limit) };
            unsafe { self.push_l(limit as u16) };
            return false;
        } else {
            return false;
        }
        while *match_len > Fse::MAX_MATCH_LEN as u32 {
            unsafe { self.push_lmd(literal_len as u16, Fse::MAX_MATCH_LEN, match_distance) };
            *match_len -= Fse::MAX_MATCH_LEN as u32;
            literal_len = 0;
            if self.lmds.len() == LMDS_PER_BLOCK as usize {
                return false;
            }
        }
        unsafe { self.push_lmd(literal_len as u16, *match_len as u16, match_distance) };
        *match_len = 0;
        true
    }

    #[inline(always)]
    unsafe fn push_l(&mut self, l: u16) {
        debug_assert!(l <= Fse::MAX_LITERAL_LEN);
        self.match_distance = 1;
        self.lmds.push_unchecked(LmdPack::<Fse>::new(l, 0, 1));
    }

    #[inline(always)]
    unsafe fn push_lmd(&mut self, l: u16, m: u16, mut d: u32) {
        debug_assert_ne!(d, 0);
        if self.match_distance == d {
            self.match_distance = d;
            d = 0;
        } else {
            self.match_distance = d;
        }
        self.lmds.push_unchecked(LmdPack::<Fse>::new(l, m, d));
        self.n_match_bytes += m as u32;
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.literals.reset();
        self.lmds.reset();
        self.n_match_bytes = 0;
        self.match_distance = 0;
    }

    #[inline(always)]
    fn n_raw_bytes(&self) -> u32 {
        self.literals.len() as u32 + self.n_match_bytes
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            literals: Literals::default(),
            lmds: Lmds::default(),
            n_match_bytes: 0,
            match_distance: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fse::Fse;
    use crate::lmd::{LmdPack, MatchDistance, MatchDistanceUnpack, MatchLen};
    use crate::lz::LzWriter;
    use crate::{fse::constants::*, lmd::DMax};

    use test_kit::{Rng, Seq};

    use super::*;

    macro_rules! test_push {
        ($name:ident, $lm:expr, $mm:expr) => {
            #[test]
            #[ignore = "expensive"]
            fn $name() -> crate::Result<()> {
                let bytes =
                    Seq::default().take(LITERALS_PER_BLOCK as usize + 0x1000).collect::<Vec<_>>();
                let mut buffer = Buffer::default();
                let mut dst_a = Vec::default();
                let mut dst_b = Vec::default();
                for seed in 0..0x0001_0000 {
                    let mut rng = Rng::new(seed);
                    let mut bytes = bytes.as_slice();
                    loop {
                        let l = (rng.gen() as usize % 0x1000 + 1).min(bytes.len());
                        let m = rng.gen() % 0x1000;
                        let d = (rng.gen() % Fse::MAX_MATCH_DISTANCE).min(dst_a.len() as u32) + 1;
                        let match_distance = MatchDistance::new(d);
                        let literals = &bytes[..l];
                        bytes = &bytes[l..];
                        let mut literals_mut = literals;
                        let mut match_len_mut = m;
                        let ok = buffer.push(&mut literals_mut, &mut match_len_mut, match_distance);
                        if ok {
                            assert_eq!(literals_mut.len(), 0);
                            assert_eq!(match_len_mut, 0);
                        }
                        let literals = &literals[..literals.len() - literals_mut.len()];
                        let mut match_len = m - match_len_mut;
                        dst_a.write_bytes_long(literals)?;
                        while match_len != 0 {
                            let match_distance_m =
                                MatchLen::new(match_len.min(Fse::MAX_MATCH_LEN as u32));
                            dst_a.write_match(match_distance_m, match_distance.into())?;
                            match_len -= match_distance_m.get();
                        }
                        if !ok {
                            break;
                        }
                    }
                    let mut match_distance = MatchDistanceUnpack::default();
                    let mut bytes = buffer.literals.as_ref();
                    for &LmdPack(literal_len_pack, match_len_pack, match_distance_pack) in
                        buffer.lmds.as_ref()
                    {
                        let literals = &bytes[..literal_len_pack.get() as usize];
                        match_distance.substitute(match_distance_pack);
                        bytes = &bytes[literal_len_pack.get() as usize..];
                        dst_b.write_bytes_long(literals)?;
                        dst_b.write_match(match_len_pack.into(), match_distance)?;
                    }
                    assert!(dst_a == dst_b);
                    buffer.reset();
                    dst_a.clear();
                    dst_b.clear();
                }
                Ok(())
            }
        };
    }

    test_push!(push_0, 0x1000, 0x1000);
    test_push!(push_1, 0x1000, 0x0010);
    test_push!(push_2, 0x0010, 0x1000);
    test_push!(push_3, 0x0010, 0x0010);
}
