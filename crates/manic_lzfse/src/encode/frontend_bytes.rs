use crate::base::MagicBytes;
use crate::error::Error;
use crate::fse::FseBackend;
use crate::lmd::MatchDistance;
use crate::match_kit;
use crate::raw::{self, RAW_HEADER_SIZE};
use crate::types::{Idx, ShortWriter};
use crate::vn::VnBackend;

use super::backend::Backend;
use super::backend_type::BackendType;
use super::constants::*;
use super::history::{History, HistoryTable, UIdx};
use super::match_object::Match;
use super::match_unit::MatchUnit;

use std::io;
use std::mem;

pub struct FrontendBytes<'a> {
    table: &'a mut HistoryTable,
    bytes: &'a [u8],
    pending: Match,
    literal_idx: Idx,
}

// Implementation notes.
//
// `i32::MAX` limit we can work around by implementing a sliding `bytes` window paired with sliding
// history values.

impl<'a> FrontendBytes<'a> {
    #[inline(always)]
    pub fn new(table: &'a mut HistoryTable, bytes: &'a [u8]) -> crate::Result<Self> {
        if bytes.len() > i32::MAX as usize {
            Err(Error::BufferOverflow)
        } else {
            Ok(Self { table, bytes, pending: Match::default(), literal_idx: Idx::default() })
        }
    }

    #[inline(always)]
    pub fn execute<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        self.init();
        self.flush(backend, dst)?;
        Ok(())
    }

    fn flush<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        // Select.
        self.flush_select(backend, dst)?;
        debug_assert_eq!(usize::from(self.literal_idx), self.bytes.len());
        // Eos.
        dst.write_short_u32(MagicBytes::Eos.into())?;
        dst.flush(true)?;
        Ok(())
    }

    fn flush_select<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        debug_assert!(self.literal_idx.is_zero());
        let len = self.bytes.len();
        if len > VN_CUTOFF as usize {
            self.flush_backend::<_, _, true>(backend, dst)
        } else if len > RAW_CUTOFF as usize {
            self.flush_backend::<_, _, false>(&mut VnBackend::default(), dst)
        } else {
            self.flush_raw(dst)
        }
    }

    fn flush_backend<B, O, const F: bool>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert!(self.literal_idx.is_zero());
        let len = self.bytes.len();
        backend.init(dst, Some(len as u32))?;
        let mark = dst.pos();
        self.finalize(backend, dst)?;
        let dst_len = (dst.pos() - mark) as u32;
        if !F && dst_len < len as u32 + RAW_HEADER_SIZE {
            return Ok(());
        }
        dst.truncate(mark);
        self.flush_raw(dst)
    }

    fn flush_raw<O>(&mut self, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        raw::raw_compress(dst, self.bytes)?;
        self.literal_idx = Idx::new(self.bytes.len() as u32);
        Ok(())
    }

    fn init(&mut self) {
        self.table.reset_idx(Idx::default() - CLAMP_INTERVAL);
        self.pending = Match::default();
        self.literal_idx = Idx::default();
    }

    fn finalize<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        self.match_short(backend, dst)?;
        self.flush_pending(backend, dst)?;
        self.flush_literals(backend, dst)?;
        backend.finalize(dst)?;
        Ok(())
    }

    // #[inline(always)]
    fn match_short<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert!(self.bytes.len() <= i32::MAX as usize);
        debug_assert!(usize::from(self.literal_idx) <= self.bytes.len());
        let len = self.bytes.len() as u32;
        if len < 4 {
            return Ok(());
        }
        let mark = Idx::new(len - 3);
        let mut idx = Idx::default();
        loop {
            // Hot loop.
            let u = unsafe { self.get_u32(idx) };
            let item = UIdx::new(u, idx);
            let queue = self.table.push::<B::Type>(item);
            let incoming = unsafe { self.find_match::<B::Type>(queue, item) };
            if let Some(select) = self.pending.select::<GOOD_MATCH_LEN>(incoming) {
                unsafe { self.push_match(backend, dst, select)? };
                if self.literal_idx >= mark {
                    // Unlikely.
                    break;
                }
                idx += 1;
                for _ in 0..(self.literal_idx - idx) {
                    let u = unsafe { self.get_u32(idx) };
                    let u_idx = UIdx::new(u, idx);
                    self.table.push::<B::Type>(u_idx);
                    idx += 1;
                }
                if idx >= mark {
                    // Unlikely
                    break;
                }
            } else {
                idx += 1;
                if idx == mark {
                    // Unlikely
                    break;
                }
            }
        }
        debug_assert!(usize::from(self.literal_idx) <= self.bytes.len());
        Ok(())
    }

    #[inline(always)]
    unsafe fn find_match<B>(&self, queue: History, item: UIdx) -> Match
    where
        B: BackendType,
    {
        let mut m = Match::default();
        for &match_idx_val in queue.iter() {
            let distance = (item.idx - match_idx_val.idx) as u32;
            debug_assert!(distance < CLAMP_INTERVAL * 3);
            if distance > B::MAX_MATCH_DISTANCE {
                break;
            }
            let match_len_inc = self.match_unit::<B>(item, match_idx_val);
            if match_len_inc > m.match_len {
                m.match_len = match_len_inc;
                m.match_idx = match_idx_val.idx;
            }
        }
        if m.match_len == 0 {
            // Likely.
            m
        } else {
            // Unlikely.
            m.idx = item.idx;
            let match_len_dec = self.match_dec::<B>(m.idx, m.match_idx);
            m.idx -= match_len_dec;
            m.match_idx -= match_len_dec;
            m.match_len += match_len_dec;
            debug_assert!(self.validate_match::<B>(m));
            m
        }
    }

    #[inline(always)]
    unsafe fn match_unit<M: MatchUnit>(&self, item: UIdx, match_item: UIdx) -> u32 {
        debug_assert!(self.validate_match_items::<M>(item, match_item));
        let len = M::match_us((item.u, match_item.u));
        if len == 4 {
            let index = usize::from(item.idx);
            let match_index = usize::from(match_item.idx);
            let max = self.bytes.len() - index;
            match_kit::fast_match_inc_unchecked(self.bytes, index, match_index, 4, max) as u32
        } else {
            len
        }
    }

    #[inline(always)]
    unsafe fn match_dec<M: MatchUnit>(&self, idx: Idx, match_idx: Idx) -> u32 {
        debug_assert!(self.validate_match_idxs::<M>(idx, match_idx));
        let index = usize::from(idx);
        let match_index = usize::from(match_idx);
        let literal_len = (idx - self.literal_idx) as usize;
        let max = (literal_len as usize).min(match_index);
        match_kit::fast_match_dec_unchecked(self.bytes, index, match_index, max) as u32
    }

    #[inline(always)]
    fn flush_pending<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
    ) -> io::Result<()> {
        if self.pending.match_len != 0 {
            unsafe { self.push_match(backend, dst, self.pending)? };
            self.pending.match_len = 0;
        }
        Ok(())
    }

    #[inline(always)]
    unsafe fn push_match<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
        m: Match,
    ) -> io::Result<()> {
        debug_assert!(self.validate_match::<B::Type>(m));
        let match_distance = MatchDistance::new((m.idx - m.match_idx) as u32);
        let literal_index = usize::from(self.literal_idx);
        let match_index = usize::from(m.idx);
        debug_assert!(literal_index <= self.bytes.len());
        debug_assert!(match_index <= self.bytes.len());
        let literals = self.bytes.get(literal_index..match_index);
        self.literal_idx = m.idx + m.match_len;
        backend.push_match(dst, literals, m.match_len, match_distance)
    }

    #[inline(always)]
    fn flush_literals<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
    ) -> io::Result<()> {
        let len = (self.bytes.len() - usize::from(self.literal_idx)) as u32;
        if len != 0 {
            unsafe { self.push_literals(backend, dst, len)? };
        }
        Ok(())
    }

    unsafe fn push_literals<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
        len: u32,
    ) -> io::Result<()> {
        debug_assert_ne!(len, 0);
        debug_assert_eq!(self.pending.match_len, 0);
        debug_assert!(usize::from(self.literal_idx + len) <= self.bytes.len());
        let index = usize::from(self.literal_idx);
        let literals = self.bytes.get(index..index + len as usize);
        self.literal_idx += len;
        backend.push_literals(dst, literals)
    }

    #[inline(always)]
    unsafe fn get_u32(&self, idx: Idx) -> u32 {
        debug_assert!(usize::from(idx) + mem::size_of::<u32>() <= self.bytes.len());
        self.bytes.as_ptr().add(usize::from(idx)).cast::<u32>().read_unaligned()
    }

    fn validate_match<B: BackendType>(&self, m: Match) -> bool {
        m.match_len != 0
            && m.match_len >= B::MATCH_UNIT
            && self.literal_idx <= m.idx
            && m.match_idx < m.idx
            && (m.idx - m.match_idx) as u32 <= B::MAX_MATCH_DISTANCE
            && usize::from(m.idx + m.match_len) <= self.bytes.len()
    }

    unsafe fn validate_match_items<M: MatchUnit>(&self, item: UIdx, match_item: UIdx) -> bool {
        self.validate_match_idxs::<M>(item.idx, match_item.idx)
            && (item.u ^ self.get_u32(item.idx)) & M::MATCH_MASK == 0
            && (match_item.u ^ self.get_u32(match_item.idx)) & M::MATCH_MASK == 0
    }

    fn validate_match_idxs<M: MatchUnit>(&self, idx: Idx, match_idx: Idx) -> bool {
        match_idx < idx && usize::from(idx) <= self.bytes.len() - M::MATCH_UNIT as usize
    }
}

#[cfg(test)]
mod tests {
    use crate::lmd::Lmd;

    use super::super::dummy::{Dummy, DummyBackend};
    use super::*;

    // Match short: zero bytes, length 4. Lower limit.
    #[test]
    fn match_short_zero_4() -> io::Result<()> {
        let mut table = HistoryTable::default();
        let bytes = vec![0u8; 4];
        let mut frontend = FrontendBytes::new(&mut table, &bytes)?;
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        frontend.table.reset_idx(Idx::default() - CLAMP_INTERVAL);
        frontend.match_short(&mut backend, &mut dst).unwrap();
        if frontend.pending.match_len != 0 {
            unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
        }
        let literal_len = frontend.bytes.len() as u32 - u32::from(frontend.literal_idx);
        if literal_len > 0 {
            unsafe { frontend.push_literals(&mut backend, &mut dst, literal_len)? };
        }
        assert_eq!(backend.literals, [0, 0, 0, 0]);
        assert_eq!(backend.lmds, vec![Lmd::<Dummy>::new(4, 0, 1)]);
        Ok(())
    }

    // Match short: zero bytes, length 5++.
    #[test]
    #[ignore = "expensive"]
    fn match_short_zero_n() -> io::Result<()> {
        let mut table = HistoryTable::default();
        let bytes = vec![0u8; 0x1000];
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        for n in 5..bytes.len() {
            backend.init(&mut dst, None)?;
            let mut frontend = FrontendBytes::new(&mut table, &bytes[..n])?;
            frontend.table.reset_idx(Idx::default() - CLAMP_INTERVAL);
            frontend.match_short(&mut backend, &mut dst)?;
            if frontend.pending.match_len != 0 {
                unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
            }
            assert_eq!(u32::from(frontend.literal_idx), frontend.bytes.len() as u32);
            assert_eq!(backend.literals, [0]);
            assert_eq!(backend.lmds, vec![Lmd::<Dummy>::new(1, n as u32 - 1, 1)]);
        }
        Ok(())
    }

    // Sandwich, incremental literals.
    #[allow(clippy::needless_range_loop)]
    #[test]
    #[ignore = "expensive"]
    fn sandwich_n_short() -> io::Result<()> {
        let mut table = HistoryTable::default();
        let mut bytes = vec![0u8; 0x1000];
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        for i in 0..4 {
            bytes[i] = i as u8 + 1;
        }
        for n in 12..bytes.len() {
            backend.init(&mut dst, None)?;
            for i in 0..4 {
                bytes[n as usize - 4 + i] = i as u8 + 1;
            }
            let mut frontend = FrontendBytes::new(&mut table, &bytes[..n])?;
            frontend.table.reset_idx(Idx::default() - CLAMP_INTERVAL);
            frontend.match_short(&mut backend, &mut dst)?;
            if frontend.pending.match_len != 0 {
                unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
            }
            assert_eq!(u32::from(frontend.literal_idx), frontend.bytes.len() as u32);
            assert_eq!(backend.literals, [1, 2, 3, 4, 0]);
            assert_eq!(
                backend.lmds,
                vec![Lmd::<Dummy>::new(5, n as u32 - 9, 1), Lmd::<Dummy>::new(0, 4, n as u32 - 4),]
            );
            bytes[n as usize - 4] = 0;
        }
        Ok(())
    }
}
