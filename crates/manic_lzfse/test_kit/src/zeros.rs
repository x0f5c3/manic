use std::io::{self, ErrorKind, Write};

pub struct Zeros;

impl Write for Zeros {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.iter().all(|&u| u == 0) {
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
