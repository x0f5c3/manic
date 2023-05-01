use crate::bits::{BitDst, BitReader, BitSrc, BitWriter};
use crate::lmd::LmdPack;
use crate::ops::WriteShort;

use super::block::LmdParam;
use super::constants::*;
use super::decoder::{self, Decoder};
use super::encoder::{self, Encoder};
use super::error_kind::FseErrorKind;
use super::object::Fse;

use std::io;

const BUF_LEN: usize = LMDS_PER_BLOCK as usize;

#[repr(C)]
pub struct Lmds(Box<[LmdPack<Fse>]>, usize);

impl Lmds {
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, lmd: LmdPack<Fse>) {
        debug_assert!(self.1 < LMDS_PER_BLOCK as usize);
        *self.0.get_mut(self.1) = lmd;
        self.1 += 1;
    }

    pub fn load<T>(&mut self, src: T, decoder: &Decoder, param: &LmdParam) -> crate::Result<()>
    where
        T: BitSrc,
    {
        let mut reader = BitReader::new(src, param.bits() as usize)?;
        let state = param.state();
        let mut state = (
            unsafe { decoder::L::new(state[0] as usize) },
            unsafe { decoder::M::new(state[1] as usize) },
            unsafe { decoder::D::new(state[2] as usize) },
        );
        let n_lmds = param.num() as usize;
        debug_assert!(n_lmds <= LMDS_PER_BLOCK as usize);
        for lmd in unsafe { self.0.get_mut(..n_lmds) } {
            // `flush` constraints:
            // 32 bit systems: flush after each L, M, D component pull.
            // 64 bit systems: flush after all L, M, D components have been pulled.
            let literal_len = unsafe { decoder.l(&mut reader, &mut state.0) };
            #[cfg(target_pointer_width = "32")]
            reader.flush();
            let match_len = unsafe { decoder.m(&mut reader, &mut state.1) };
            #[cfg(target_pointer_width = "32")]
            reader.flush();
            let match_distance_zeroed = unsafe { decoder.d(&mut reader, &mut state.2) };
            reader.flush();
            *lmd = LmdPack(literal_len.into(), match_len.into(), match_distance_zeroed);
        }
        reader.finalize()?;
        if state != (decoder::L::default(), decoder::M::default(), decoder::D::default()) {
            return Err(FseErrorKind::BadLmdPayload.into());
        }
        self.1 = n_lmds;
        Ok(())
    }

    pub fn store<T>(&self, dst: &mut T, encoder: &Encoder) -> io::Result<LmdParam>
    where
        T: BitDst + WriteShort,
    {
        debug_assert!(self.1 <= LMDS_PER_BLOCK as usize);
        let mark = dst.pos();
        // 8 byte pad.
        dst.write_short_u64(0)?;
        let n_bytes = (self.1 * MAX_LMD_BITS as usize + 7) / 8;
        let mut writer = BitWriter::new(dst, n_bytes)?;
        let mut state = (encoder::L::default(), encoder::M::default(), encoder::D::default());
        for &LmdPack(literal_len, match_len, match_distance_zeroed) in
            unsafe { self.0.get(..self.1).iter().rev() }
        {
            // `flush` constraints:
            // 32 bit systems: flush after each L, M, D component pull.
            // 64 bit systems: flush after all L, M, D components have been pulled.
            unsafe { encoder.d(&mut writer, &mut state.2, match_distance_zeroed) };
            #[cfg(target_pointer_width = "32")]
            writer.flush();
            unsafe { encoder.m(&mut writer, &mut state.1, match_len.into()) };
            #[cfg(target_pointer_width = "32")]
            writer.flush();
            unsafe { encoder.l(&mut writer, &mut state.0, literal_len.into()) };
            writer.flush();
        }
        let state =
            [u32::from(state.0) as u16, u32::from(state.1) as u16, u32::from(state.2) as u16];
        let bits = writer.finalize()? as u32;
        let n_payload_bytes = (dst.pos() - mark) as u32;
        Ok(LmdParam::new(self.1 as u32, n_payload_bytes, bits, state).expect("internal error"))
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        debug_assert!(self.1 <= LMDS_PER_BLOCK as usize);
        self.1
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        debug_assert!(self.1 <= LMDS_PER_BLOCK as usize);
        self.1 = 0;
    }
}

impl AsRef<[LmdPack<Fse>]> for Lmds {
    #[inline(always)]
    fn as_ref(&self) -> &[LmdPack<Fse>] {
        debug_assert!(self.1 <= LMDS_PER_BLOCK as usize);
        &self.0[..self.1]
    }
}

impl Default for Lmds {
    fn default() -> Self {
        Self(vec![LmdPack::default(); BUF_LEN].into_boxed_slice(), 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::bits::ByteBits;
    use crate::fse::Weights;

    use test_kit::Rng;

    use super::*;

    /// Test buddy.
    struct Buddy {
        weights: Weights,
        encoder: Encoder,
        decoder: Decoder,
        src: Lmds,
        dst: Lmds,
        param: LmdParam,
        enc: Vec<u8>,
    }

    impl Buddy {
        #[allow(dead_code)]
        pub fn push(&mut self, lmds: &[LmdPack<Fse>]) {
            assert!(lmds.len() <= LMDS_PER_BLOCK as usize);
            self.src.reset();
            lmds.iter().for_each(|&u| unsafe { self.src.push_unchecked(u) });
        }

        fn encode(&mut self) -> io::Result<()> {
            self.weights.load(self.src.as_ref(), &[]);
            self.encoder.init(&self.weights);
            self.enc.clear();
            self.param = self.src.store(&mut self.enc, &self.encoder)?;
            assert_eq!(self.enc.len(), self.param.n_payload_bytes() as usize);
            Ok(())
        }

        fn decode(&mut self) -> io::Result<()> {
            self.decoder.init(&self.weights);
            self.dst.load(ByteBits::new(&self.enc), &self.decoder, &self.param)?;
            Ok(())
        }

        fn check(&self) -> bool {
            self.src.as_ref() == self.dst.as_ref()
        }

        fn check_encode_decode(&mut self, lmds: &[LmdPack<Fse>]) -> io::Result<bool> {
            self.push(lmds);
            self.encode()?;
            self.decode()?;
            Ok(self.check())
        }
    }

    impl Default for Buddy {
        fn default() -> Self {
            Self {
                weights: Weights::default(),
                encoder: Encoder::default(),
                decoder: Decoder::default(),
                src: Lmds::default(),
                dst: Lmds::default(),
                param: LmdParam::default(),
                enc: Vec::default(),
            }
        }
    }

    #[test]
    fn empty() -> io::Result<()> {
        let mut buddy = Buddy::default();
        assert!(buddy.check_encode_decode(&[])?);
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn literal_len() -> io::Result<()> {
        let mut buddy = Buddy::default();
        let mut lmds = Vec::default();
        for literal_len in 0..=MAX_L_VALUE {
            lmds.push(LmdPack::new(literal_len, 0, 1));
        }
        assert!(buddy.check_encode_decode(&lmds)?);
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn match_len() -> io::Result<()> {
        let mut buddy = Buddy::default();
        let mut lmds = Vec::default();
        for match_len in 0..=MAX_M_VALUE {
            lmds.push(LmdPack::new(0, match_len, 1));
        }
        assert!(buddy.check_encode_decode(&lmds)?);
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn match_distance() -> io::Result<()> {
        let mut buddy = Buddy::default();
        let mut lmds = Vec::default();
        for i in (0..=MAX_D_VALUE).step_by(LMDS_PER_BLOCK as usize) {
            lmds.clear();
            for match_distance in i..(i + LMDS_PER_BLOCK).min(MAX_D_VALUE) {
                lmds.push(LmdPack::new(0, 0, match_distance));
            }
            assert!(buddy.check_encode_decode(&lmds)?);
        }
        Ok(())
    }

    // Random LMD.
    #[test]
    #[ignore = "expensive"]
    fn rng_1() -> crate::Result<()> {
        let mut buddy = Buddy::default();
        let mut lmds = Vec::default();
        for seed in 0..0x8000 {
            let mut rng = Rng::new(seed);
            lmds.clear();
            for _ in 0..seed.min(LMDS_PER_BLOCK) {
                let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
                let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
                let d = ((rng.gen() & 0x0000_0FFF) * (MAX_D_VALUE as u32 + 1)) >> 12;
                lmds.push(LmdPack::new(l as u16, m as u16, d));
            }
            assert!(buddy.check_encode_decode(&lmds)?);
        }
        Ok(())
    }

    // Bitwise mutation. We are looking to break the decoder. In all cases the
    // decoder should reject invalid data via `Err(error)` and exit gracefully. It should not hang/
    // segfault/ panic/ trip debug assertions or break in a any other fashion.
    #[test]
    #[ignore = "expensive"]
    fn mutate_1() -> crate::Result<()> {
        let mut buddy = Buddy::default();
        let mut lmds = Vec::default();
        for seed in 0..0x0100 {
            let mut rng = Rng::new(seed);
            lmds.clear();
            for _ in 0..seed.min(LMDS_PER_BLOCK) {
                let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
                let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
                let d = ((rng.gen() & 0x0000_0FFF) * (MAX_D_VALUE as u32 + 1)) >> 12;
                lmds.push(LmdPack::new(l as u16, m as u16, d));
            }
            assert!(buddy.check_encode_decode(&lmds)?);
            for index in 0..buddy.enc.len() {
                for n_bit in 0..8 {
                    let bit = 1 << n_bit;
                    buddy.enc[index] ^= bit;
                    let _ = buddy.decode();
                    buddy.enc[index] ^= bit;
                }
            }
            assert!(buddy.check_encode_decode(&lmds)?);
        }
        Ok(())
    }

    // Byte mutation. We are looking to break the decoder. In all cases the
    // decoder should reject invalid data via `Err(error)` and exit gracefully. It should not hang/
    // segfault/ panic/ trip debug assertions or break in a any other fashion.
    #[test]
    #[ignore = "expensive"]
    fn mutate_2() -> crate::Result<()> {
        let mut buddy = Buddy::default();
        let mut lmds = Vec::default();
        for seed in 0..0x0080 {
            let mut rng = Rng::new(seed);
            lmds.clear();
            for _ in 0..seed.min(LMDS_PER_BLOCK) {
                let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
                let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
                let d = ((rng.gen() & 0x0000_0FFF) * (MAX_D_VALUE as u32 + 1)) >> 12;
                lmds.push(LmdPack::new(l as u16, m as u16, d));
            }
            assert!(buddy.check_encode_decode(&lmds)?);
            for index in 0..buddy.enc.len() {
                for byte in 0..=0xFF {
                    buddy.enc[index] ^= byte;
                    let _ = buddy.decode();
                    buddy.enc[index] ^= byte;
                }
            }
            assert!(buddy.check_encode_decode(&lmds)?);
        }
        Ok(())
    }
}
