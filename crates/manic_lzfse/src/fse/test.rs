use crate::base::MagicBytes;
use crate::error::Error;
use crate::lmd::{self, Lmd};
use crate::ops::{PatchInto, Skip, WriteShort};
use crate::test_utils;
use crate::types::{Idx, ShortBuffer};

use test_kit::{Rng, Seq};

use super::block::{FseBlock, LiteralParam, LmdParam};
use super::fse_core::FseCore;
use super::Fse;
use super::{constants::*, FseBackend};

use std::io::{self};

// Block level tests.

/// Test buddy.
struct Buddy {
    backend: FseBackend,
    core: FseCore,
    enc: Vec<u8>,
    dec: Vec<u8>,
}

impl Buddy {
    fn encode_lmds(&mut self, literals: &[u8], lmds: &[Lmd<Fse>]) -> io::Result<(u32, u32)> {
        test_utils::encode_lmds(&mut self.enc, &mut self.backend, literals, lmds)
    }

    fn decode(&mut self) -> crate::Result<(u32, u32)> {
        self.dec.clear();
        let mut src = self.enc.as_slice();
        let n_header_bytes = self.core.load_v2(src)?;
        src.skip(n_header_bytes as usize);
        let n_literal_bytes = self.core.load_literals(src)?;
        src.skip(n_literal_bytes as usize);
        let n_lmd_bytes = self.core.decode(&mut self.dec, src)?;
        src.skip(n_lmd_bytes as usize);
        let n_raw_bytes = self.dec.len() as u32;
        Ok((n_header_bytes + n_literal_bytes + n_lmd_bytes, n_raw_bytes))
    }

    fn decode_n(&mut self, n: u32) -> crate::Result<(u32, u32)> {
        self.dec.clear();
        let mut src = self.enc.as_slice();
        let n_header_bytes = self.core.load_v2(src)?;
        src.skip(n_header_bytes as usize);
        let n_literal_bytes = self.core.load_literals(src)?;
        src.skip(n_literal_bytes as usize);
        let n_lmd_bytes = self.core.load_lmds(src)?;
        src.skip(n_lmd_bytes as usize);
        self.core.decode_n_init(&self.dec);
        while self.core.decode_n(&mut self.dec, n)? {}
        let n_raw_bytes = self.dec.len() as u32;
        Ok((n_header_bytes + n_literal_bytes + n_lmd_bytes, n_raw_bytes))
    }

    fn check_encode_decode(&mut self, literals: &[u8], lmds: &[Lmd<Fse>]) -> crate::Result<bool> {
        let (e_raw_bytes, e_payload_bytes) = self.encode_lmds(literals, lmds)?;
        let (d_payload_bytes, d_raw_bytes) = self.decode()?;
        Ok(e_raw_bytes == d_raw_bytes
            && e_payload_bytes == d_payload_bytes
            && self.check_lmds(literals, lmds))
    }

    fn check_encode_decode_n(
        &mut self,
        literals: &[u8],
        lmds: &[Lmd<Fse>],
        n: u32,
    ) -> crate::Result<bool> {
        let (e_raw_bytes, e_payload_bytes) = self.encode_lmds(literals, lmds)?;
        let (d_payload_bytes, d_raw_bytes) = self.decode_n(n)?;
        Ok(e_raw_bytes == d_raw_bytes
            && e_payload_bytes == d_payload_bytes
            && self.check_lmds(literals, lmds))
    }

    fn check_lmds(&self, literals: &[u8], lmds: &[Lmd<Fse>]) -> bool {
        test_utils::check_lmds(&self.dec, literals, lmds)
    }

    fn mutate_n_raw_bytes(&mut self, delta: i32) -> crate::Result<bool> {
        let mut block = FseBlock::default();
        let (n_header_bytes, n_weight_payload_bytes) =
            block.load_v2_short(self.enc.as_slice().short_bytes())?;
        let n_raw_bytes = (block.n_raw_bytes() as i32 + delta) as u32;
        let literal = *block.literal();
        let lmd = *block.lmd();
        if let Ok(block) = FseBlock::new(n_raw_bytes, literal, lmd) {
            let bytes = self.enc.patch_into(Idx::default(), n_header_bytes as usize);
            block.store_v2(bytes, n_weight_payload_bytes);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn mutate_v2_literal_n_payload_bytes(&mut self, delta: i32) -> crate::Result<bool> {
        let mut block = FseBlock::default();
        let (n_header_bytes, n_weight_payload_bytes) =
            block.load_v2_short(self.enc.as_slice().short_bytes())?;
        let n_raw_bytes = block.n_raw_bytes();
        let literal = *block.literal();
        if let Ok(literal) = LiteralParam::new(
            literal.num(),
            (literal.n_payload_bytes() as i32 + delta) as u32,
            literal.bits(),
            *literal.state(),
        ) {
            let lmd = *block.lmd();
            if let Ok(block) = FseBlock::new(n_raw_bytes, literal, lmd) {
                let bytes = self.enc.patch_into(Idx::default(), n_header_bytes as usize);
                block.store_v2(bytes, n_weight_payload_bytes);
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    fn mutate_v2_lmd_n_payload_bytes(&mut self, delta: i32) -> crate::Result<bool> {
        let mut block = FseBlock::default();
        let (n_header_bytes, n_weight_payload_bytes) =
            block.load_v2_short(self.enc.as_slice().short_bytes())?;
        let n_raw_bytes = block.n_raw_bytes();
        let lmd = *block.lmd();
        if let Ok(lmd) = LmdParam::new(
            lmd.num(),
            (lmd.n_payload_bytes() as i32 + delta) as u32,
            lmd.bits(),
            *lmd.state(),
        ) {
            let literal = *block.literal();
            if let Ok(block) = FseBlock::new(n_raw_bytes, literal, lmd) {
                let bytes = self.enc.patch_into(Idx::default(), n_header_bytes as usize);
                block.store_v2(bytes, n_weight_payload_bytes);
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
}

impl Default for Buddy {
    fn default() -> Self {
        Self {
            backend: FseBackend::default(),
            core: FseCore::default(),
            enc: Vec::default(),
            dec: Vec::default(),
        }
    }
}

// Quote.
#[test]
fn quote() -> crate::Result<()> {
    let bytes = b"Full fathom five thy father lies; \
                 Of his bones are coral made; \
                 Those are pearls that were his eyes: \
                 Nothing of him that doth fade; \
                 But doth suffer a sea-change; \
                 Into something rich and strange."; // William Shakespeare. The Tempest.
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    lmd::split_lmd(&mut lmds, bytes.len() as u32, 0, 1);
    assert!(buddy.check_encode_decode(bytes.as_ref(), &lmds)?);
    Ok(())
}

// Incremental literal len.
#[test]
#[ignore = "expensive"]
fn literals() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        assert!(buddy.check_encode_decode(&bytes[..literal_len], &lmds)?);
    }
    Ok(())
}

// Small literal len, match len and match distance.
#[test]
#[ignore = "expensive"]
fn matches_1() -> crate::Result<()> {
    let bytes = Seq::default().take(0x0100).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        for match_len in 3..literal_len as u32 {
            for match_distance in 1..literal_len as u32 {
                lmds.clear();
                lmd::split_lmd(&mut lmds, literal_len as u32, match_len, match_distance);
                assert!(buddy.check_encode_decode(&bytes[..literal_len], &lmds)?);
            }
        }
    }
    Ok(())
}

// Small match len, all match distance.
#[test]
#[ignore = "expensive"]
fn matches_2() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for match_len in 3..0x10 {
        for match_distance in 1..bytes.len() {
            let literal_len = match_distance;
            lmds.clear();
            lmd::split_lmd(&mut lmds, literal_len as u32, match_len, match_distance as u32);
            assert!(buddy.check_encode_decode(&bytes[..literal_len], &lmds)?);
        }
    }
    Ok(())
}

// Coarse match len, all match distance.
#[test]
#[ignore = "expensive"]
fn matches_3() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for match_len in (0x10..0x100).step_by(0x10) {
        for match_distance in 1..bytes.len() {
            let literal_len = match_distance;
            lmds.clear();
            lmd::split_lmd(&mut lmds, literal_len as u32, match_len, match_distance as u32);
            assert!(buddy.check_encode_decode(&bytes[..literal_len], &lmds)?);
        }
    }
    Ok(())
}

// Coarse match len, all match distance.
#[test]
#[ignore = "expensive"]
fn matches_4() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for match_len in (0x100..0x1000).step_by(0x100) {
        for match_distance in 1..bytes.len() {
            let literal_len = match_distance;
            lmds.clear();
            lmd::split_lmd(&mut lmds, literal_len as u32, match_len, match_distance as u32);
            assert!(buddy.check_encode_decode(&bytes[..literal_len], &lmds)?);
        }
    }
    Ok(())
}

// Random LMD generation.
#[test]
#[ignore = "expensive"]
fn fuzz_lmd() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for i in 0..0x8000 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push(Lmd::new(1, 0, 1));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
            let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
            let d = ((rng.gen() & 0x0000_0FFF) * (MAX_D_VALUE as u32 + 1)) >> 12;
            if bytes.len() < index + l as usize || lmds.len() == LMDS_PER_BLOCK as usize {
                break;
            }
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push(Lmd::new(l, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        assert!(buddy.check_encode_decode(&bytes[..index as usize], &lmds)?);
    }
    Ok(())
}

// Random LMD generation with n decoding.
#[test]
#[ignore = "expensive"]
fn fuzz_lmd_n() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for i in 0..0x8000 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push(Lmd::new(1, 0, 1));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
            let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
            let d = ((rng.gen() & 0x0000_0FFF) * (MAX_D_VALUE as u32 + 1)) >> 12;
            if bytes.len() < index + l as usize || lmds.len() == LMDS_PER_BLOCK as usize {
                break;
            }
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push(Lmd::new(l, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        assert!(buddy.check_encode_decode_n(&bytes[..index as usize], &lmds, 1 + i)?);
    }
    Ok(())
}

// Mutate `literal.n_payload_bytes` +1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn edge_1() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        buddy.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !buddy.mutate_v2_lmd_n_payload_bytes(1)? {
            continue;
        }
        match buddy.decode() {
            Err(_) => {}
            Ok(_) => panic!(),
        }
    }
    Ok(())
}

// Mutate `literal.n_payload_bytes` -1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn edge_2() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        buddy.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !buddy.mutate_v2_lmd_n_payload_bytes(-1)? {
            continue;
        }
        match buddy.decode() {
            Err(_) => {}
            Ok(_) => panic!(),
        }
    }
    Ok(())
}

// Mutate `literal.n_payload_bytes` +1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn edge_3() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        buddy.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !buddy.mutate_v2_literal_n_payload_bytes(1)? {
            continue;
        }
        match buddy.decode() {
            Err(_) => {}
            Ok(_) => panic!(),
        }
    }
    Ok(())
}

// Mutate `literal.n_payload_bytes` -1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn edge_4() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        buddy.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !buddy.mutate_v2_literal_n_payload_bytes(-1)? {
            continue;
        }
        match buddy.decode() {
            Err(_) => {}
            Ok(_) => panic!(),
        }
    }
    Ok(())
}

// Mutate `n_raw_bytes` +1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn edge_5() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        buddy.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !buddy.mutate_n_raw_bytes(1)? {
            continue;
        }
        match buddy.decode() {
            Err(Error::Fse(super::FseErrorKind::BadLmdPayload)) => {}
            _ => panic!(),
        }
    }
    Ok(())
}

// Mutate `n_raw_bytes` -1. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn edge_6() -> crate::Result<()> {
    let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for literal_len in 1..bytes.len() {
        lmds.clear();
        lmd::split_lmd(&mut lmds, literal_len as u32, 0, 1);
        buddy.encode_lmds(&bytes[..literal_len], &lmds)?;
        if !buddy.mutate_n_raw_bytes(-1)? {
            continue;
        }
        match buddy.decode() {
            Err(Error::Fse(super::FseErrorKind::BadLmdPayload)) => {}
            _ => panic!(),
        }
    }
    Ok(())
}

// Random payload generation with mutations. We are looking to break the decoder. In all cases the
// decoder should reject invalid data via `Err(error)` and exit gracefully. It should not hang/
// segfault/ panic/ trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_rng_1() -> crate::Result<()> {
    let bytes = Seq::default().take(0x1000).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for i in 0..0x0200 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push(Lmd::new(1, 0, 1));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
            let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
            let d = ((rng.gen() & 0x0000_0FFF) * (MAX_D_VALUE as u32 + 1)) >> 12;
            if bytes.len() < index + l as usize {
                break;
            }
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push(Lmd::new(l, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        buddy.encode_lmds(&bytes[..index as usize], &lmds)?;
        for j in 4..buddy.enc.len() {
            buddy.enc[j] = buddy.enc[j].wrapping_add(1);
            let _ = buddy.decode();
            buddy.enc[j] = buddy.enc[j].wrapping_sub(2);
            let _ = buddy.decode();
            buddy.enc[j] = buddy.enc[j].wrapping_add(1);
        }
    }
    Ok(())
}

// Random payload generation with mutations. We are looking to break the decoder. In all cases the
// decoder should reject invalid data via `Err(error)` and exit gracefully. It should not hang/
// segfault/ panic/ trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn mutate_rng_2() -> crate::Result<()> {
    let bytes = Seq::default().take(0x1000).collect::<Vec<_>>();
    let mut buddy = Buddy::default();
    let mut lmds = Vec::default();
    for i in 0..0x0200 {
        let mut rng = Rng::new(i);
        lmds.clear();
        lmds.push(Lmd::new(1, 0, 1));
        let mut index = 1;
        let mut n_raw_bytes = index as u32;
        loop {
            let l = ((rng.gen() & 0x0000_FFFF) * (MAX_L_VALUE as u32 + 1)) >> 16;
            let m = ((rng.gen() & 0x0000_FFFF) * (MAX_M_VALUE as u32 + 1)) >> 16;
            let d = ((rng.gen() & 0x0000_0FFF) * (MAX_D_VALUE as u32 + 1)) >> 12;
            if bytes.len() < index + l as usize {
                break;
            }
            let m = m.max(3);
            let d = d.min(n_raw_bytes).max(1);
            lmds.push(Lmd::new(l, m, d));
            index += l as usize;
            n_raw_bytes += m;
        }
        buddy.encode_lmds(&bytes[..index as usize], &lmds)?;
        for _ in 0..255 {
            for j in 4..buddy.enc.len() {
                buddy.enc[j] = buddy.enc[j].wrapping_add(1);
                let _ = buddy.decode();
            }
        }
    }
    Ok(())
}

// Random noise for payload. We are looking to break the decoder. In all cases the decoder should
// reject invalid data via `Err(error)` and exit gracefully. It should not hang/ segfault/ panic/
// trip debug assertions or break in a any other fashion.
#[test]
#[ignore = "expensive"]
fn fuzz_noise() -> crate::Result<()> {
    let mut buddy = Buddy::default();
    for i in 0..0x0100_0000 {
        let mut seq = Seq::new(Rng::new(i));
        buddy.enc.write_short_u32(MagicBytes::Vx2.into())?;
        buddy.enc.resize(0x400, 0);
        for i in 0x0004..0x0400 {
            buddy.enc[i] = seq.next().unwrap();
        }
        let _ = buddy.decode();
    }
    Ok(())
}
