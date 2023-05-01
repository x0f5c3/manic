use crate::base::MagicBytes;
use crate::error::Error;
use crate::fse::FseCore;
use crate::ops::{Len, Pos};
use crate::raw::RawBlock;
use crate::ring::{RingBlock, RingLzWriter, RingSize};
use crate::types::{ByteReader, Idx};
use crate::vn::VnCore;

use super::constants::*;

use std::convert::TryInto;
use std::io::{self, Read, Sink};

#[derive(Debug, PartialEq, Eq)]
enum State {
    None,
    Fse,
    Vn,
    Raw,
    Eos,
    Err,
}

pub struct ReaderCore<'a, I: for<'b> ByteReader<'b>> {
    ring: RingLzWriter<'a, Sink, Output>,
    inner: I,
    fse_core: &'a mut FseCore,
    vn_core: VnCore,
    raw_block: RawBlock,
    state: State,
    idx: Idx,
}

impl<'a, I: for<'b> ByteReader<'b>> ReaderCore<'a, I> {
    #[allow(clippy::assertions_on_constants)]
    pub fn new(ring: RingLzWriter<'a, Sink, Output>, inner: I, fse_core: &'a mut FseCore) -> Self {
        assert!(Output::RING_BLK_SIZE <= Output::RING_SIZE / 4);
        Self {
            ring,
            inner,
            fse_core,
            vn_core: VnCore::default(),
            raw_block: RawBlock::default(),
            state: State::None,
            idx: Idx::default(),
        }
    }

    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<'a, I: for<'b> ByteReader<'b>> ReaderCore<'a, I> {
    #[cold]
    #[rustfmt::skip]
    fn fill(&mut self) -> crate::Result<bool> {
        debug_assert_eq!(self.idx, self.ring.pos());
        self.inner.fill()?;
        match self.state {
            State::None => self.init()?,
            State::Fse  => self.fill_fse()?,
            State::Vn   => self.fill_vn()?,
            State::Raw  => self.fill_raw()?,
            State::Eos  => return Ok(false),
            State::Err  => return Err(Error::BadReaderState),
        };
        // TODO consider formalizing fill overflow limits.
        assert!(self.ring.pos() - self.idx < Output::RING_SIZE as i32  / 2);
        Ok(true)
    }

    fn fill_fse(&mut self) -> crate::Result<()> {
        debug_assert_eq!(self.idx, self.ring.pos());
        let len = Output::RING_BLK_SIZE;
        if !self.fse_core.decode_n(&mut self.ring, len)? {
            self.state = State::None;
        }
        Ok(())
    }

    fn fill_vn(&mut self) -> crate::Result<()> {
        debug_assert_eq!(self.idx, self.ring.pos());
        let len = Output::RING_BLK_SIZE;
        if !self.vn_core.decode_n(&mut self.ring, &mut self.inner, len)? {
            self.state = State::None;
        }
        Ok(())
    }

    fn fill_raw(&mut self) -> crate::Result<()> {
        debug_assert_eq!(self.idx, self.ring.pos());
        let len = Output::RING_BLK_SIZE;
        if !self.raw_block.decode_n(&mut self.ring, &mut self.inner, len)? {
            self.state = State::None;
        }
        Ok(())
    }

    fn init(&mut self) -> crate::Result<()> {
        debug_assert_eq!(self.state, State::None);
        self.inner.fill()?;
        if self.inner.len() < 4 {
            return Err(crate::Error::PayloadUnderflow);
        }
        let magic_bytes: MagicBytes = self.inner.peek_u32().try_into()?;
        match magic_bytes {
            MagicBytes::Vx1 => self.init_vx1(),
            MagicBytes::Vx2 => self.init_vx2(),
            MagicBytes::Vxn => self.init_vxn(),
            MagicBytes::Raw => self.init_raw(),
            MagicBytes::Eos => self.init_eos(),
        }
    }

    fn init_vx1(&mut self) -> crate::Result<()> {
        let view = self.inner.view();
        let n = self.fse_core.load_v1(view)?;
        self.inner.skip(n as usize);
        self.init_vx1_vx2_cont()
    }

    fn init_vx2(&mut self) -> crate::Result<()> {
        let view = self.inner.view();
        let n = self.fse_core.load_v2(view)?;
        self.inner.skip(n as usize);
        self.init_vx1_vx2_cont()
    }

    fn init_vx1_vx2_cont(&mut self) -> crate::Result<()> {
        let view = self.inner.view();
        let n = self.fse_core.load_literals(view)?;
        self.inner.skip(n as usize);
        let view = self.inner.view();
        let n = self.fse_core.load_lmds(view)?;
        self.inner.skip(n as usize);
        self.fse_core.decode_n_init(&self.ring);
        self.state = State::Fse;
        Ok(())
    }

    fn init_vxn(&mut self) -> crate::Result<()> {
        let view = self.inner.view();
        let n = self.vn_core.load_short(view)?;
        self.inner.skip(n as usize);
        self.state = State::Vn;
        Ok(())
    }

    fn init_raw(&mut self) -> crate::Result<()> {
        let view = self.inner.view();
        let n = self.raw_block.load_short(view)?;
        self.inner.skip(n as usize);
        self.state = State::Raw;
        Ok(())
    }

    fn init_eos(&mut self) -> crate::Result<()> {
        if self.inner.len() != 4 || !self.inner.is_eof() {
            self.state = State::Err;
            Err(crate::Error::PayloadOverflow)
        } else {
            self.state = State::Eos;
            Ok(())
        }
    }
}

impl<'a, I: for<'b> ByteReader<'b>> Read for ReaderCore<'a, I> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mark = self.idx;
        loop {
            debug_assert!(self.idx <= self.ring.pos());
            let limit = ((self.ring.pos() - self.idx) as usize).min(buf.len());
            self.ring.copy(unsafe { buf.get_mut(..limit) }, self.idx);
            self.idx += limit as u32;
            buf = unsafe { buf.get_mut(limit..) };
            if buf.is_empty() {
                break;
            }
            if !self.fill()? {
                break;
            }
        }
        Ok((self.idx - mark) as usize)
    }
}
