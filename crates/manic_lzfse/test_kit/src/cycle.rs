use std::io::prelude::*;
use std::io::{self, ErrorKind};

/// Byte sequence generator that that repeats 0, 1, 2, ..., 255 cycles.
#[derive(Copy, Clone, Debug)]
pub struct Cycle(u8);

impl Default for Cycle {
    fn default() -> Self {
        Self(0)
    }
}

impl Iterator for Cycle {
    type Item = u8;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.0 = self.0.wrapping_add(1);
        Some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl Read for Cycle {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for b in buf.iter_mut() {
            *b = self.next().unwrap();
        }
        Ok(buf.len())
    }
}

impl Write for Cycle {
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
