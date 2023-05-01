use crate::base::MagicBytes;
use crate::decode::Take;
use crate::kit::PackBits;
use crate::ops::{Len, ReadData, WriteData};
use crate::types::ShortBuffer;

use super::constants::*;
use super::error_kind::FseErrorKind;

/// Theoretical maximum `n_raw_bytes` given the supplied parameters.
pub fn n_raw_bytes_limit(n_literals: u32, n_lmds: u32) -> u32 {
    assert!(n_literals <= LITERALS_PER_BLOCK);
    assert!(n_lmds <= LMDS_PER_BLOCK);
    n_literals + n_lmds * MAX_M_VALUE as u32
}

/// Naive maximum `n_payload_bytes` given the supplied parameters with leeway.
fn lmd_n_payload_bytes_limit(num: u32) -> u32 {
    1024 + 8 + (num * MAX_L_BITS as u32 + num * MAX_M_BITS as u32 + num * MAX_D_BITS as u32 + 7) / 8
}

/// Naive maximum `n_payload_bytes` given the supplied parameters with leeway.
pub fn literal_n_payload_bytes_limit(num: u32) -> u32 {
    1024 + (num * MAX_U_BITS as u32 + 7) / 8
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FseBlock {
    literal: LiteralParam,
    lmd: LmdParam,
    n_raw_bytes: u32,
}

impl FseBlock {
    pub fn new(n_raw_bytes: u32, literal: LiteralParam, lmd: LmdParam) -> crate::Result<Self> {
        if n_raw_bytes > n_raw_bytes_limit(literal.num(), lmd.num()) {
            Err(FseErrorKind::BadRawByteCount.into())
        } else {
            Ok(Self { literal, lmd, n_raw_bytes })
        }
    }

    pub fn load_v1_short<I: Copy + ShortBuffer>(
        &mut self,
        mut src: I,
    ) -> crate::Result<(u32, u32)> {
        assert!(V1_HEADER_SIZE <= I::SHORT_LIMIT);
        self.load_v1(src.take(V1_HEADER_SIZE)?.short_bytes())
    }

    pub fn load_v2_short<I: Copy + ShortBuffer>(
        &mut self,
        mut src: I,
    ) -> crate::Result<(u32, u32)> {
        assert!(V2_HEADER_SIZE <= I::SHORT_LIMIT);
        self.load_v2(src.take(V2_HEADER_SIZE)?.short_bytes())
    }

    pub fn load_v1(&mut self, src: &[u8]) -> crate::Result<(u32, u32)> {
        match self.load_v1_internal(src) {
            Ok(u) => Ok(u),
            Err(e) => {
                *self = Self::default();
                Err(e)
            }
        }
    }

    pub fn load_v2(&mut self, src: &[u8]) -> crate::Result<(u32, u32)> {
        match self.load_v2_internal(src) {
            Ok(u) => Ok(u),
            Err(e) => {
                *self = Self::default();
                Err(e)
            }
        }
    }

    #[rustfmt::skip]
    pub fn load_v1_internal(&mut self, src: &[u8]) -> crate::Result<(u32, u32)> {
        let mut src = &src[..V1_HEADER_SIZE as usize];
        let magic_bytes               = src.read_u32();
        assert_eq!(magic_bytes, MagicBytes::Vx1.into());
        self.n_raw_bytes              = src.read_u32();
        let n_payload_bytes           = src.read_u32();
        self.literal.num              = src.read_u32();
        self.lmd.num                  = src.read_u32();
        self.literal.n_payload_bytes  = src.read_u32();
        self.lmd.n_payload_bytes      = src.read_u32();
        self.literal.bits             = src.read_u32().wrapping_neg();
        self.literal.state[0]         = src.read_u16();
        self.literal.state[1]         = src.read_u16();
        self.literal.state[2]         = src.read_u16();
        self.literal.state[3]         = src.read_u16();
        self.lmd.bits                 = src.read_u32().wrapping_neg();
        self.lmd.state[0]             = src.read_u16();
        self.lmd.state[1]             = src.read_u16();
        self.lmd.state[2]             = src.read_u16();
        if n_payload_bytes < self.literal.n_payload_bytes.wrapping_add(self.lmd.n_payload_bytes) {
            return Err(FseErrorKind::BadPayloadCount.into());
        }
        self.validate()?;
        Ok((V1_HEADER_SIZE, V1_WEIGHT_PAYLOAD_BYTES))
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[rustfmt::skip]
    fn load_v2_internal(&mut self, src: &[u8]) -> crate::Result<(u32, u32)> {
        let mut src = &src[..V2_HEADER_SIZE as usize];
        let magic_bytes              =     src.read_u32();
        assert_eq!(magic_bytes, MagicBytes::Vx2.into());
        self.n_raw_bytes             =     src.read_u32();
        let p                        =     src.read_u64();
        self.literal.num             =     p.get_bits(00, 20) as u32;
        self.literal.n_payload_bytes =     p.get_bits(20, 20) as u32;
        self.lmd.num                 =     p.get_bits(40, 20) as u32;
        self.literal.bits            = 7 - p.get_bits(60, 03) as u32;
        let p                        =     src.read_u64();
        self.literal.state[0]        =     p.get_bits(00, 10) as u16;
        self.literal.state[1]        =     p.get_bits(10, 10) as u16;
        self.literal.state[2]        =     p.get_bits(20, 10) as u16;
        self.literal.state[3]        =     p.get_bits(30, 10) as u16;
        self.lmd.n_payload_bytes     =     p.get_bits(40, 20) as u32;
        self.lmd.bits                = 7 - p.get_bits(60, 3) as u32;
        let p                        =     src.read_u64();
        let header_size              =     p.get_bits(00, 32) as u32;
        self.lmd.state[0]            =     p.get_bits(32, 10) as u16;
        self.lmd.state[1]            =     p.get_bits(42, 10) as u16;
        self.lmd.state[2]            =     p.get_bits(52, 10) as u16;
        let n_weight_payload_bytes = header_size.wrapping_sub(V2_HEADER_SIZE);
        if n_weight_payload_bytes > V2_WEIGHT_PAYLOAD_BYTES_MAX {
            return Err(FseErrorKind::BadWeightPayload.into());
        }
        self.validate()?;
        Ok((V2_HEADER_SIZE, n_weight_payload_bytes))
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[allow(dead_code)]
    #[rustfmt::skip]
    pub fn store_v1(&self,mut dst: &mut [u8]) {
        assert_eq!(dst.len(), V1_HEADER_SIZE as usize);
        let literal_param = self.literal();
        let literal_state = literal_param.state();
        let lmd_param = self.lmd();
        let lmd_state = lmd_param.state();
        dst.write_u32(MagicBytes::Vx1.into());
        dst.write_u32(self.n_raw_bytes);
        dst.write_u32(self.n_payload_bytes());
        dst.write_u32(literal_param.num());
        dst.write_u32(lmd_param.num());
        dst.write_u32(literal_param.n_payload_bytes());
        dst.write_u32(lmd_param.n_payload_bytes());
        dst.write_u32(literal_param.bits().wrapping_neg());
        dst.write_u16(literal_state[0]);
        dst.write_u16(literal_state[1]);
        dst.write_u16(literal_state[2]);
        dst.write_u16(literal_state[3]);
        dst.write_u32(lmd_param.bits().wrapping_neg());
        dst.write_u16(lmd_state[0]);
        dst.write_u16(lmd_state[1]);
        dst.write_u16(lmd_state[2]);
        debug_assert_eq!(dst.len(), 0);
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[rustfmt::skip]
    pub fn store_v2(&self,mut dst: &mut [u8], n_weight_payload_bytes: u32) {
        assert_eq!(dst.len(), V2_HEADER_SIZE as usize);
        let literal_state = self.literal.state();
        let lmd_state = self.lmd.state();
        let header_size = V2_HEADER_SIZE + n_weight_payload_bytes;
        dst.write_u32(MagicBytes::Vx2.into());
        dst.write_u32(self.n_raw_bytes as u32);
        let mut p: u64 = 0;
        p.set_bits(00, 20,     self.literal.num() as u64);
        p.set_bits(20, 20,     self.literal.n_payload_bytes() as u64);
        p.set_bits(40, 20,     self.lmd.num() as u64);
        p.set_bits(60, 03, 7 - self.literal.bits() as u64);
        dst.write_u64(p);
        let mut p: u64 = 0;
        p.set_bits(00, 10,     literal_state[0] as u64);
        p.set_bits(10, 10,     literal_state[1] as u64);
        p.set_bits(20, 10,     literal_state[2] as u64);
        p.set_bits(30, 10,     literal_state[3] as u64);
        p.set_bits(40, 20,     self.lmd.n_payload_bytes() as u64);
        p.set_bits(60, 03, 7 - self.lmd.bits() as u64);
        dst.write_u64(p);
        let mut p: u64 = 0;
        p.set_bits(00, 32,     header_size as u64);
        p.set_bits(32, 10,     lmd_state[0] as u64);
        p.set_bits(42, 10,     lmd_state[1] as u64);
        p.set_bits(52, 10,     lmd_state[2] as u64);
        dst.write_u64(p);
        debug_assert_eq!(dst.len(), 0);
    }

    #[inline(always)]
    pub fn literal(&self) -> &LiteralParam {
        &self.literal
    }

    #[inline(always)]
    pub fn n_payload_bytes(&self) -> u32 {
        self.literal.n_payload_bytes() + self.lmd.n_payload_bytes()
    }

    #[inline(always)]
    pub fn lmd(&self) -> &LmdParam {
        &self.lmd
    }

    #[inline(always)]
    pub fn n_raw_bytes(&self) -> u32 {
        self.n_raw_bytes
    }

    fn validate(&self) -> crate::Result<()> {
        self.lmd.validate()?;
        self.literal.validate()?;
        if self.n_raw_bytes > n_raw_bytes_limit(self.literal.num, self.lmd.num) {
            return Err(FseErrorKind::BadRawByteCount.into());
        }

        Ok(())
    }
}

impl Default for FseBlock {
    fn default() -> Self {
        Self { literal: LiteralParam::default(), lmd: LmdParam::default(), n_raw_bytes: 0 }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LmdParam {
    num: u32,
    n_payload_bytes: u32,
    bits: u32,
    state: [u16; 3],
}

// Implementation notes:
//
// We maintain the structure in a valid state at all times.
impl LmdParam {
    pub fn new(num: u32, n_payload_bytes: u32, bits: u32, state: [u16; 3]) -> crate::Result<Self> {
        let s = Self { num, n_payload_bytes, bits, state };
        s.validate()?;
        Ok(s)
    }

    #[inline(always)]
    pub fn num(&self) -> u32 {
        self.num
    }

    #[inline(always)]
    pub fn n_payload_bytes(&self) -> u32 {
        self.n_payload_bytes
    }

    #[inline(always)]
    pub fn bits(&self) -> u32 {
        self.bits
    }

    #[inline(always)]
    pub fn state(&self) -> &[u16; 3] {
        &self.state
    }

    fn validate(&self) -> crate::Result<()> {
        if self.num > LMDS_PER_BLOCK
            || self.n_payload_bytes < 8
            || self.n_payload_bytes > lmd_n_payload_bytes_limit(self.num)
        {
            Err(FseErrorKind::BadLmdCount(self.num).into())
        } else if self.bits > 7 {
            Err(FseErrorKind::BadLmdBits.into())
        } else if (self.state[0] as u32) >= L_STATES
            || (self.state[1] as u32) >= M_STATES
            || (self.state[2] as u32) >= D_STATES
        {
            Err(FseErrorKind::BadLmdState.into())
        } else {
            Ok(())
        }
    }
}

impl Default for LmdParam {
    fn default() -> Self {
        Self { num: 0, n_payload_bytes: 0, bits: 0, state: [0; 3] }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LiteralParam {
    num: u32,
    n_payload_bytes: u32,
    bits: u32,
    state: [u16; 4],
}

// Implementation notes:
//
// We maintain the structure in a valid state at all times.
impl LiteralParam {
    pub fn new(num: u32, n_payload_bytes: u32, bits: u32, state: [u16; 4]) -> crate::Result<Self> {
        let s = Self { num, n_payload_bytes, bits, state };
        s.validate()?;
        Ok(s)
    }

    #[inline(always)]
    pub fn num(&self) -> u32 {
        self.num
    }

    #[inline(always)]
    pub fn n_payload_bytes(&self) -> u32 {
        self.n_payload_bytes
    }

    #[inline(always)]
    pub fn bits(&self) -> u32 {
        self.bits
    }

    #[inline(always)]
    pub fn state(&self) -> &[u16; 4] {
        &self.state
    }

    fn validate(&self) -> crate::Result<()> {
        if self.num % 4 != 0
            || self.num > LITERALS_PER_BLOCK
            || self.n_payload_bytes > literal_n_payload_bytes_limit(self.num)
        {
            Err(FseErrorKind::BadLiteralCount(self.num).into())
        } else if self.bits > 7 {
            Err(FseErrorKind::BadLiteralBits.into())
        } else if (self.state[0] as u32) >= U_STATES
            || (self.state[1] as u32) >= U_STATES
            || (self.state[2] as u32) >= U_STATES
            || (self.state[3] as u32) >= U_STATES
        {
            Err(FseErrorKind::BadLmdPayload.into())
        } else {
            Ok(())
        }
    }
}

impl Default for LiteralParam {
    fn default() -> Self {
        Self { num: 0, n_payload_bytes: 0, bits: 0, state: [0; 4] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_block() -> FseBlock {
        let literals =
            LiteralParam::new(LITERALS_PER_BLOCK, LITERALS_PER_BLOCK, 7, [U_STATES as u16 - 1; 4])
                .unwrap();
        let lmds = LmdParam::new(
            LMDS_PER_BLOCK,
            LMDS_PER_BLOCK,
            7,
            [L_STATES as u16 - 1, M_STATES as u16 - 1, D_STATES as u16 - 1],
        )
        .unwrap();
        FseBlock::new(LITERALS_PER_BLOCK, literals, lmds).unwrap()
    }

    #[test]
    fn v1_store_load() -> crate::Result<()> {
        let block_1 = dummy_block();
        let mut bs = [0u8; V1_HEADER_SIZE as usize];
        block_1.store_v1(&mut bs);
        let mut block_2 = FseBlock::default();
        let (n_header_payload_bytes, n_weight_payload_bytes) = block_2.load_v1(bs.as_ref())?;
        assert_eq!(n_header_payload_bytes, V1_HEADER_SIZE);
        assert_eq!(n_weight_payload_bytes, V1_WEIGHT_PAYLOAD_BYTES);
        assert_eq!(block_1, block_2);
        Ok(())
    }

    #[test]
    fn v2_store_load() -> crate::Result<()> {
        let block_1 = dummy_block();
        let mut bs = [0u8; V2_HEADER_SIZE as usize];
        block_1.store_v2(&mut bs, V2_WEIGHT_PAYLOAD_BYTES_MAX);
        let mut block_2 = FseBlock::default();
        let (n_header_payload_bytes, n_weight_payload_bytes) = block_2.load_v2(bs.as_ref())?;
        assert_eq!(n_header_payload_bytes, V2_HEADER_SIZE);
        assert_eq!(n_weight_payload_bytes, V2_WEIGHT_PAYLOAD_BYTES_MAX);
        assert_eq!(block_1, block_2);
        Ok(())
    }
}
