use std::io;
use std::io::prelude::*;

pub trait ReadExtFully {
    fn read_fully(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

impl<R: Read> ReadExtFully for R {
    #[inline(always)]
    fn read_fully(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;
        while n < buf.len() {
            match self.read(&mut buf[n..]) {
                Ok(0) => break,
                Ok(len) => n += len,
                Err(e) => return Err(e),
            }
        }
        Ok(n)
    }
}
