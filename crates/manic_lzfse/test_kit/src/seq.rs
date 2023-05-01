use super::rng::Rng;

use std::io::prelude::*;
use std::io::{self, ErrorKind};

/// Byte sequence generator with Rng core.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Seq {
    rng: Rng,
    mask: u32,
    u: u32,
    n: u32,
}

impl Seq {
    pub fn new(rng: Rng) -> Self {
        Self { rng, mask: 0xFFFF_FFFF, u: 0, n: 0 }
    }

    pub fn masked(rng: Rng, mask: u32) -> Self {
        Self { rng, mask, u: 0, n: 0 }
    }

    #[inline(always)]
    pub fn gen(&mut self) -> u8 {
        if self.n == 0 {
            self.u = self.rng.gen() & self.mask;
            self.n = 4;
        }
        self.n -= 1;
        let v = self.u as u8;
        self.u >>= 8;
        v
    }
}

impl Default for Seq {
    fn default() -> Self {
        Self::new(Rng::default())
    }
}

impl Iterator for Seq {
    type Item = u8;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.gen())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl Read for Seq {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for b in buf.iter_mut() {
            *b = self.next().unwrap();
        }
        Ok(buf.len())
    }
}

impl Write for Seq {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.iter().all(|&u| Some(u) == self.next()) {
            Ok(buf.len())
        } else {
            Err(ErrorKind::InvalidData.into())
        }
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
