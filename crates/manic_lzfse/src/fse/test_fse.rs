use crate::bits::{BitReader, BitWriter, ByteBits};

use test_kit::{Rng, Seq};

use super::decoder::{self, UEntry};
use super::encoder::{self, EEntry};
use super::weights;

use std::io::{self};

const N_SYMBOLS: u32 = 0x0100;
const N_STATES: u32 = 0x0400;
const MAX_BITS: usize = 0x0A;

// Low level FSE encode/ decode.

/// Test buddy.
struct Buddy {
    weights: [u16; N_SYMBOLS as usize],
    encode_table: [EEntry; N_SYMBOLS as usize],
    decode_table: [UEntry; N_STATES as usize],
    bytes: Vec<u8>,
    enc: Vec<u8>,
    dec: Vec<u8>,
    state: u32,
    off: usize,
}

impl Buddy {
    fn push(&mut self, byte: u8) {
        self.bytes.push(byte);
        self.weights[byte as usize] += 1;
    }

    fn encode(&mut self) -> io::Result<()> {
        weights::normalize_m1(&mut self.weights, self.bytes.len() as u32, N_STATES);
        encoder::build_e_table(&self.weights, N_STATES, &mut self.encode_table);
        self.enc.clear();
        self.enc.resize(8, 0);
        let allocate = (self.bytes.len() * MAX_BITS + 7) / 8;
        let mut wtr = BitWriter::new(&mut self.enc, allocate)?;
        let mut state = N_STATES;
        for b in self.bytes.iter().rev() {
            debug_assert!(N_STATES <= state);
            debug_assert!(state < 2 * N_STATES);
            unsafe { self.encode_table[*b as usize].encode(&mut wtr, &mut state) };
            wtr.flush();
        }
        self.off = wtr.finalize()?;
        self.state = state - N_STATES;
        Ok(())
    }

    fn decode(&mut self) -> io::Result<()> {
        decoder::build_u_table(&self.weights, &mut self.decode_table);
        self.dec.clear();
        self.dec.resize(self.bytes.len(), 0);
        let mut rdr = BitReader::new(ByteBits::new(&self.enc), self.off)?;
        let mut state = self.state as usize;
        for b in self.dec.iter_mut() {
            debug_assert!(state < N_STATES as usize);
            *b = unsafe { self.decode_table[state].decode(&mut rdr, &mut state) };
            rdr.flush();
        }
        rdr.finalize()?;
        if state == 0 {
            Ok(())
        } else {
            Err(io::ErrorKind::InvalidData.into())
        }
    }

    fn check(&self) -> bool {
        self.bytes == self.dec
    }

    fn check_encode_decode(&mut self, bytes: &[u8]) -> io::Result<bool> {
        self.reset();
        bytes.iter().for_each(|&u| self.push(u));
        self.encode()?;
        self.decode()?;
        Ok(self.check())
    }

    fn reset(&mut self) {
        self.weights = [0; N_SYMBOLS as usize];
        self.bytes.clear();
        self.off = 0;
        self.state = 0;
    }
}

impl Default for Buddy {
    fn default() -> Self {
        Self {
            weights: [0; 256],
            encode_table: [EEntry::default(); N_SYMBOLS as usize],
            decode_table: [UEntry::default(); N_STATES as usize],
            bytes: Vec::default(),
            enc: Vec::default(),
            dec: Vec::default(),
            state: 0,
            off: 0,
        }
    }
}

// Empty.
#[test]
fn empty() -> io::Result<()> {
    let mut buddy = Buddy::default();
    assert!(buddy.check_encode_decode(&[])?);
    Ok(())
}

// Quote.
#[test]
fn quote() -> io::Result<()> {
    let bytes = b"Full fathom five thy father lies; \
    Of his bones are coral made; \
    Those are pearls that were his eyes: \
    Nothing of him that doth fade; \
    But doth suffer a sea-change; \
    Into something rich and strange."; // William Shakespeare. The Tempest.
    let mut buddy = Buddy::default();
    assert!(buddy.check_encode_decode(bytes)?);
    Ok(())
}

// Flat.
#[test]
fn flat() -> io::Result<()> {
    let mut buddy = Buddy::default();
    for n in 0..0x10 {
        buddy.reset();
        for i in 0..0xFF {
            for _ in 0..n {
                buddy.push(i);
            }
        }
        buddy.encode()?;
        buddy.decode()?;
        assert!(buddy.check());
    }
    Ok(())
}

// Random bytes.
#[test]
#[ignore = "expensive"]
fn rng_1() -> io::Result<()> {
    let mut buddy = Buddy::default();
    for n in 0..0x0001_0000 {
        let mut seq = Seq::new(Rng::new(n));
        buddy.reset();
        for _ in 0..n {
            buddy.push(seq.gen());
        }
        buddy.encode()?;
        buddy.decode()?;
        assert!(buddy.check());
    }
    Ok(())
}

// Random bytes, incremental entropy.
#[test]
#[ignore = "expensive"]
fn rng_2() -> io::Result<()> {
    let mut buddy = Buddy::default();
    for n in 0..0x0001_0000 {
        let mask = n * 0x0001_0001;
        let mut seq = Seq::masked(Rng::new(n), mask);
        buddy.reset();
        for _ in 0..n {
            buddy.push(seq.gen());
        }
        buddy.encode()?;
        buddy.decode()?;
        assert!(buddy.check());
    }
    Ok(())
}

#[test]
#[ignore = "expensive"]
fn interleave() -> io::Result<()> {
    let mut buddy = Buddy::default();
    let mut bytes = Vec::default();
    for i in 0..N_STATES as usize {
        for j in 0..N_STATES as usize {
            bytes.clear();
            bytes.resize(bytes.len() + 4 * i, 0);
            bytes.resize(bytes.len() + 4 * j, 1);
            assert!(buddy.check_encode_decode(&bytes)?);
        }
    }
    Ok(())
}

#[test]
#[ignore = "expensive"]
fn interleave_balanced() -> io::Result<()> {
    let mut buddy = Buddy::default();
    let mut bytes = Vec::default();
    for i in 0..N_STATES as usize {
        for j in 0..N_STATES as usize {
            bytes.clear();
            bytes.resize(bytes.len() + 4 * i, 0);
            bytes.resize(bytes.len() + 4 * j, 1);
            if i < j {
                bytes.resize(bytes.len() + 4 * (j - i), 0);
            } else {
                bytes.resize(bytes.len() + 4 * (i - j), 1);
            };
            assert!(buddy.check_encode_decode(&bytes)?);
        }
    }
    Ok(())
}
