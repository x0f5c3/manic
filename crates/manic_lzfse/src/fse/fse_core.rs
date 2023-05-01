use crate::bits::{AsBitSrc, BitReader, BitSrc};
use crate::decode::Take;
use crate::kit::W00;
use crate::lmd::{LiteralLen, LmdPack, MatchDistanceUnpack, MatchLen};
use crate::lz::LzWriter;
use crate::types::{ShortBuffer, ShortBytes};

use super::block::FseBlock;
use super::constants::*;
use super::decoder::{self, Decoder};
use super::error_kind::FseErrorKind;
use super::literals::Literals;
use super::lmds::Lmds;
use super::object::Fse;
use super::weights::Weights;

pub struct FseCore {
    decoder: Decoder,
    literals: Literals,
    lmds: Lmds,
    block: FseBlock,
    weights: Weights,
    literal_index: u32,
    lmd_index: u32,
    mark: u64,
    match_distance: MatchDistanceUnpack<Fse>,
}

// Implementation notes:
//
// BitSrc requires an 8 byte pad. The LMD payload is padded. The literal payload is not padded, so
// we borrow 8 bytes from the header.

impl FseCore {
    pub fn load_v1<I>(&mut self, mut src: I) -> crate::Result<u32>
    where
        I: Copy + ShortBuffer,
    {
        assert!(V1_WEIGHT_PAYLOAD_BYTES <= I::SHORT_LIMIT);
        let (n_header_payload_bytes, n_weight_payload_bytes) = self.block.load_v1_short(src)?;
        src.skip(n_header_payload_bytes as usize);
        let payload = src.take(n_weight_payload_bytes)?;
        self.weights.load_v1(payload.short_bytes())?;
        self.decoder.init(&self.weights);
        Ok(n_header_payload_bytes + n_weight_payload_bytes - 8)
    }

    pub fn load_v2<I>(&mut self, mut src: I) -> crate::Result<u32>
    where
        I: Copy + ShortBuffer,
    {
        assert!(V2_WEIGHT_PAYLOAD_BYTES_MAX <= I::SHORT_LIMIT);
        let (n_header_payload_bytes, n_weight_payload_bytes) = self.block.load_v2_short(src)?;
        src.skip(n_header_payload_bytes as usize);
        let payload = src.take(n_weight_payload_bytes)?;
        self.weights.load_v2(payload.short_bytes())?;
        self.decoder.init(&self.weights);
        Ok(n_header_payload_bytes + n_weight_payload_bytes - 8)
    }

    pub fn load_literals<I>(&mut self, mut src: I) -> crate::Result<u32>
    where
        I: AsBitSrc + Copy + ShortBuffer,
    {
        let payload = src.take(self.n_literal_payload_bytes())?;
        let bits = payload.as_bit_src();
        self.literals.load(bits, &self.decoder, self.block.literal())?;
        Ok(self.n_literal_payload_bytes())
    }

    pub fn load_lmds<I>(&mut self, mut src: I) -> crate::Result<u32>
    where
        I: AsBitSrc + Copy + ShortBuffer,
    {
        let payload = src.take(self.n_lmd_payload_bytes())?;
        let bits = payload.as_bit_src();
        self.lmds.load(bits, &self.decoder, self.block.lmd())?;
        Ok(self.n_lmd_payload_bytes())
    }

    pub fn decode<O, I>(&mut self, dst: &mut O, mut src: I) -> crate::Result<u32>
    where
        O: LzWriter,
        I: AsBitSrc + Copy + ShortBuffer,
    {
        let payload = src.take(self.n_lmd_payload_bytes())?;
        let bits = payload.as_bit_src();
        self.decode_internal(dst, bits)?;
        Ok(self.n_lmd_payload_bytes())
    }

    #[inline(always)]
    fn decode_internal<O: LzWriter, T: BitSrc>(&self, dst: &mut O, src: T) -> crate::Result<()> {
        let mut reader = BitReader::new(src, self.block.lmd().bits() as usize)?;
        let state = self.block.lmd().state();
        let mut state = (
            unsafe { decoder::L::new(state[0] as usize) },
            unsafe { decoder::M::new(state[1] as usize) },
            unsafe { decoder::D::new(state[2] as usize) },
        );
        let mut literal_index = 0;
        let mut n_match_bytes = 0;
        let mut match_distance = MatchDistanceUnpack::default();
        let mut n = self.block.lmd().num();
        while n != 0 {
            // `flush` constraints:
            // 32 bit systems: flush after each L, M, D component pull.
            // 64 bit systems: flush after all L, M, D components have been pulled.
            let literal_len = unsafe { self.decoder.l(&mut reader, &mut state.0) };
            #[cfg(target_pointer_width = "32")]
            reader.flush();
            let match_len = unsafe { self.decoder.m(&mut reader, &mut state.1) };
            #[cfg(target_pointer_width = "32")]
            reader.flush();
            let match_distance_pack = unsafe { self.decoder.d(&mut reader, &mut state.2) };
            reader.flush();
            match_distance.substitute(match_distance_pack);
            let ptr = unsafe { self.literals.as_ptr().add(literal_index as usize) };
            let bytes = unsafe { ShortBytes::from_raw_parts(ptr, literal_len.get() as usize) };
            literal_index += literal_len.get();
            if literal_index <= LITERALS_PER_BLOCK {
                // Likely.
                dst.write_bytes_short::<LiteralLen<Fse>, W00>(bytes)?;
                if match_len.get() != 0 {
                    n_match_bytes += match_len.get();
                    dst.write_match::<Fse>(match_len, match_distance)?;
                }
            } else {
                // Unlikely.
                return Err(FseErrorKind::BadLmdPayload.into());
            }
            n -= 1;
        }
        reader.finalize()?;
        if literal_index <= self.block.literal().num()
            && n_match_bytes + literal_index == self.block.n_raw_bytes()
            && state == (decoder::L::default(), decoder::M::default(), decoder::D::default())
        {
            Ok(())
        } else {
            Err(FseErrorKind::BadLmdPayload.into())
        }
    }

    pub fn decode_n_init<O: LzWriter>(&mut self, dst: &O) {
        self.literal_index = 0;
        self.lmd_index = 0;
        self.mark = dst.n_raw_bytes();
        self.match_distance = MatchDistanceUnpack::default();
    }

    pub fn decode_n<O: LzWriter>(&mut self, dst: &mut O, n: u32) -> crate::Result<bool> {
        self.decode_n_internal(dst, dst.n_raw_bytes() + n as u64)?;
        Ok(self.lmd_index != self.block.lmd().num())
    }

    #[inline(always)]
    fn decode_n_internal<O: LzWriter>(&mut self, dst: &mut O, dst_mark: u64) -> crate::Result<()> {
        let mut literal_index = self.literal_index;
        let mut lmd_index = self.lmd_index;
        let mut match_distance = self.match_distance;
        let lmds = self.lmds.as_ref();
        while dst.n_raw_bytes() <= dst_mark {
            if lmd_index == self.block.lmd().num() {
                // Unlikely
                if literal_index <= self.block.literal().num()
                    && (dst.n_raw_bytes() - self.mark) as u32 == self.block.n_raw_bytes()
                {
                    // Likely
                    break;
                } else {
                    // Unlikely
                    return Err(FseErrorKind::BadLmdPayload.into());
                }
            }
            let &LmdPack(literal_len_pack, match_len_pack, match_distance_pack) =
                unsafe { lmds.get(lmd_index as usize) };
            let literal_len: LiteralLen<Fse> = literal_len_pack.into();
            let match_len: MatchLen<Fse> = match_len_pack.into();
            match_distance.substitute(match_distance_pack);
            let ptr = unsafe { self.literals.as_ptr().add(literal_index as usize) };
            let bytes = unsafe { ShortBytes::from_raw_parts(ptr, literal_len.get() as usize) };
            literal_index += literal_len.get() as u32;
            if literal_index <= LITERALS_PER_BLOCK {
                // Likely.
                dst.write_bytes_short::<LiteralLen<Fse>, W00>(bytes)?;
                if match_len.get() != 0 {
                    dst.write_match::<Fse>(match_len, match_distance)?;
                }
            } else {
                // Unlikely
                return Err(FseErrorKind::BadLmdPayload.into());
            }
            lmd_index += 1;
        }
        self.literal_index = literal_index;
        self.lmd_index = lmd_index;
        self.match_distance = match_distance;
        Ok(())
    }

    #[inline(always)]
    fn n_literal_payload_bytes(&self) -> u32 {
        self.block.literal().n_payload_bytes() + 8
    }

    #[inline(always)]
    fn n_lmd_payload_bytes(&self) -> u32 {
        self.block.lmd().n_payload_bytes()
    }
}

impl Default for FseCore {
    fn default() -> Self {
        Self {
            literals: Literals::default(),
            lmds: Lmds::default(),
            weights: Weights::default(),
            decoder: Decoder::default(),
            block: FseBlock::default(),
            literal_index: 0,
            lmd_index: 0,
            mark: 0,
            match_distance: MatchDistanceUnpack::default(),
        }
    }
}
