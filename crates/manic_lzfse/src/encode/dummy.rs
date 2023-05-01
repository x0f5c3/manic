use crate::encode::{Backend, BackendType, MatchUnit};
use crate::lmd::{DMax, LMax, Lmd, LmdMax, MMax, MatchDistance};
use crate::lz::LzWriter;
use crate::ops::WriteLong;
use crate::types::{ShortBuffer, ShortWriter};

use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dummy;

impl LMax for Dummy {
    const MAX_LITERAL_LEN: u16 = u16::MAX;
}

impl MMax for Dummy {
    const MAX_MATCH_LEN: u16 = u16::MAX;
}

impl DMax for Dummy {
    const MAX_MATCH_DISTANCE: u32 = 0x3FFF_FFFF;
}

impl LmdMax for Dummy {}

impl MatchUnit for Dummy {
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

impl BackendType for Dummy {}

#[derive(Debug)]
pub struct DummyBackend {
    pub literals: Vec<u8>,
    pub lmds: Vec<Lmd<Dummy>>,
}

impl DummyBackend {
    #[allow(dead_code)]
    pub fn decode<W: LzWriter>(&self, dst: &mut W) -> crate::Result<()> {
        let mut index = 0;
        for &lmd in self.lmds.iter() {
            let literal_len = lmd.0.get() as usize;
            dst.write_bytes_long(&self.literals[index..index + literal_len])?;
            dst.write_match(lmd.1, lmd.2.into())?;
            index += literal_len;
        }
        Ok(())
    }
}

impl Backend for DummyBackend {
    type Type = Dummy;

    fn init<O: ShortWriter>(&mut self, _: &mut O, _: Option<u32>) -> io::Result<()> {
        self.literals.clear();
        self.lmds.clear();
        Ok(())
    }

    fn push_literals<I: ShortBuffer, O: ShortWriter>(
        &mut self,
        dst: &mut O,
        literals: I,
    ) -> io::Result<()> {
        self.push_match(dst, literals, 0, MatchDistance::new(1))
    }

    fn push_match<I: ShortBuffer, O: ShortWriter>(
        &mut self,
        _: &mut O,
        literals: I,
        match_len: u32,
        match_distance: MatchDistance<Self::Type>,
    ) -> io::Result<()> {
        assert!(literals.len() <= u32::MAX as usize);
        let literal_len = literals.len() as u32;
        self.literals.write_long(literals)?;
        self.lmds.push(Lmd::new(literal_len, match_len, match_distance.get()));
        Ok(())
    }

    fn finalize<O: ShortWriter>(&mut self, _: &mut O) -> io::Result<()> {
        Ok(())
    }
}

impl Default for DummyBackend {
    fn default() -> Self {
        Self { literals: Vec::default(), lmds: Vec::default() }
    }
}
