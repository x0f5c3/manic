use crate::bits::AsBitSrc;
use crate::ops::{Len, PeekData, Skip};

use super::short_buffer::ShortBuffer;

use std::io;

// Implementation notes:
//
// https://github.com/rust-lang/rfcs/blob/master/text/1598-generic_associated_types.md

pub trait ByteReader<'a>: Len + PeekData + Skip {
    const VIEW_LIMIT: usize;

    type View: 'a + AsBitSrc + Copy + ShortBuffer;

    fn fill(&mut self) -> io::Result<()>;

    fn view(&'a self) -> Self::View;

    /// True if the buffer has no source or the buffer source is empty. Note that the buffer may
    /// still contain residual data.
    fn is_eof(&self) -> bool;

    fn is_full(&self) -> bool;
}

impl<'a, 'b> ByteReader<'a> for &'b [u8] {
    const VIEW_LIMIT: usize = usize::MAX as usize;

    type View = &'a [u8];

    #[inline(always)]
    fn fill(&mut self) -> io::Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn view(&'a self) -> Self::View {
        self
    }

    #[inline(always)]
    fn is_eof(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_full(&self) -> bool {
        true
    }
}
