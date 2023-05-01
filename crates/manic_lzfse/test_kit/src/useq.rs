use std::io::prelude::*;
use std::io::{self, ErrorKind};

/// Generates up to 10_923_528 bytes with the constraint that all possible 4 byte subslices are
/// unique.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Useq {
    u: [u8; 4],
    n: u8,
}

impl Iterator for Useq {
    type Item = u8;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 4 {
            self.u[2] = self.u[2].wrapping_add(1);
            if self.u[2] == 0 {
                self.u[1] += 1;
                self.u[2] = self.u[1] + 1;
                if self.u[1] == 0xFE {
                    self.u[0] += 1;
                    if self.u[0] == 0xFD {
                        return None;
                    }
                    self.u[1] = self.u[0] + 1;
                    self.u[2] = self.u[1] + 1;
                }
            }
            self.n = 0;
        }
        let v = self.u[self.n as usize];
        self.n += 1;
        Some(v)
    }
}

impl Default for Useq {
    fn default() -> Self {
        Self { u: [1, 2, 3, 0], n: 0 }
    }
}

impl Read for Useq {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for b in buf.iter_mut() {
            *b = self.next().ok_or(io::ErrorKind::UnexpectedEof)?;
        }
        Ok(buf.len())
    }
}

impl Write for Useq {
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
