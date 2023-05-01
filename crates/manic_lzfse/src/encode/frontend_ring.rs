use crate::base::MagicBytes;
use crate::fse::{Fse, FseBackend};
use crate::kit::ReadExtFully;
use crate::lmd::DMax;
use crate::lmd::MatchDistance;
use crate::raw::{self, RAW_HEADER_SIZE};
use crate::ring::{self, Ring, RingBlock, RingType};
use crate::types::{Idx, ShortWriter};
use crate::vn::{Vn, VnBackend};

use super::backend::Backend;
use super::backend_type::BackendType;
use super::constants::*;
use super::history::{History, HistoryTable, UIdx};
use super::match_object::Match;
use super::match_unit::MatchUnit;

use crate::bits::BitDst;
use std::io::{self, ErrorKind, Read};

const OVERMATCH_LEN: u32 = 4 + ring::OVERMATCH_LEN as u32;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Commit {
    Fse,
    Vn,
    None,
}

pub struct FrontendRing<'a, T> {
    table: &'a mut HistoryTable,
    ring: Ring<'a, T>,
    commit: Commit,
    pending: Match,
    head: Idx,
    literal_idx: Idx,
    idx: Idx,
    tail: Idx,
    mark: Idx,
    clamp: Idx,
    n_raw_bytes: u64,
}

// Implementation notes:
//
// Built over `Ring`. It may be easier to visualize if we imagine a sliding window over flat input
// data as opposed to being overly bogged down in `Ring` internal mechanics.
//
// <------------------------------------ INPUT DATA--------------------------------------->
//              |--X--|---------------------- W ----------------------|--X--|
//                                           | ----- G ----- |
//                    ^H          ^L              ^I                  ^T    ^U
//                                               |---- M ----|
//                          |---- R ----|
//                                            |--- P ---|
//                       |--- Q ---|
//              <--------------------------- RING -------------------------->
//
//
// -------------------------------------------------------------------------------------------
// Tag | Description | Notes
// -------------------------------------------------------------------------------------------
// W   | Window      | Window/ working buffer.
// H   | Window head |
// T   | Window tail |
// U   | Mark        | Fill mark.
// X   | Undefined   | Undefined buffer data.
// L   | Literal idx | Data below this point has been pushed into the backend.
// I   | Working idx | Data below this point has been pushed into history but not the backend.
// G   | Goldilocks  | G = RING_SIZE / 2..RING_SIZE / 2 + RING_BLK_SIZE
// M   | Match       | Incoming match target.
// R   | Match       | Incoming match source.
// P   | Match       | Pending match target.
// Q   | Match       | Pending match source.
//
//
// Global constraints:
// H <= L <= I <= T <= U <= H + RING_SIZE
// H <= R < M <= T      R can overlap M
// H <= Q < P <= T      Q can overlap P
// P < M                P can overlap M
// U % RING_BLK_SIZE == 0
//
// Operational constraints:
// I == L == H == T     If and only if: init.
// I == L == H          If and only if: no blocks processed.
//
// Match constraints:
// source.idx <  target.idx
// source.len == target.len
//
// 'Goldilocks` zone is the optimal working index position with respect to matching. Within the zone
// we have RING_SIZE / 2 - RING_BLOCK_SIZE incremental matching and RING_SIZE /2 decremental
// matching.
//
// `commit` defines the type of history values we have stored in our `table`. More specifically the
// minimum match length and index hash method. Fse and Vn history types are incompatible in this
// regard. The initial `commit` value is `None` and the compression logic may commit to `Fse` or
// `Vn` but not both.
//
// `clamp` defines the `Idx` value, with 1GB leeway, at which distances should be clamped in our
// `table`. Failure to do this will result in old values eventually, that is after 3GB or so,
// wrapping back around resulting in data corruption. We shall keep well away from these limits.
//
// Performance. The `match_long` and `match_short` loops are very hot. We sacrifice some awkwardness
// for improved performance.

impl<'a, T: Copy + RingBlock> FrontendRing<'a, T> {
    // Non flush max match len that doesn't overshoot our tail.
    const MAX_MATCH_LEN: u32 = T::RING_SIZE / 2 - T::RING_BLK_SIZE - OVERMATCH_LEN;

    pub fn new(ring: Ring<'a, T>, table: &'a mut HistoryTable) -> Self {
        assert!(T::RING_BLK_SIZE.is_power_of_two());
        assert!(Vn::MAX_MATCH_DISTANCE < T::RING_SIZE / 2);
        assert!(Fse::MAX_MATCH_DISTANCE < T::RING_SIZE / 2);
        assert!(T::RING_SIZE <= CLAMP_INTERVAL / 4);
        assert!(0x100 < T::RING_BLK_SIZE as usize);
        assert!(T::RING_BLK_SIZE <= T::RING_SIZE / 4);
        assert!(OVERMATCH_LEN < T::RING_LIMIT);
        let zero = Idx::default();
        Self {
            table,
            ring,
            commit: Commit::None,
            pending: Match::default(),
            head: zero,
            literal_idx: zero,
            idx: zero,
            tail: zero,
            mark: zero,
            clamp: zero,
            n_raw_bytes: 0,
        }
    }

    /// Call after init, otherwise behavior is undefined.
    #[inline(always)]
    pub fn copy<B, I, O>(&mut self, backend: &mut B, dst: &mut O, src: &mut I) -> io::Result<u64>
    where
        B: Backend,
        I: Read,
        O: ShortWriter,
    {
        loop {
            if !self.copy_block(src)? {
                break;
            }
            self.match_block(backend, dst)?;
        }
        Ok(self.n_raw_bytes)
    }

    #[inline(always)]
    fn copy_block<I: Read>(&mut self, src: &mut I) -> io::Result<bool> {
        debug_assert!(self.validate_global());
        debug_assert_eq!(self.tail % T::RING_BLK_SIZE, 0);
        let index = self.tail % T::RING_SIZE as usize;
        let bytes =
            self.ring.get_mut(index..index + T::RING_BLK_SIZE as usize).ok_or(io::Error::new(
                ErrorKind::InvalidInput,
                "Failed to copy_block due to self.ring.get_mut failing",
            ))?;
        let len = src.read_fully(bytes)?;
        self.tail += len as u32;
        self.n_raw_bytes += len as u64;
        Ok(len == T::RING_BLK_SIZE as usize)
    }

    /// Call after init, otherwise behavior is undefined.
    pub fn write<O>(
        &mut self,
        backend: &mut FseBackend,
        mut src: &[u8],
        dst: &mut O,
    ) -> io::Result<usize>
    where
        O: ShortWriter,
    {
        let total_len = src.len();
        loop {
            if !self.write_block(&mut src) {
                break;
            }
            self.match_block(backend, dst)?;
        }
        Ok(total_len)
    }

    #[inline(always)]
    fn write_block(&mut self, src: &mut &[u8]) -> bool {
        debug_assert!(self.validate_global());
        let len = src.len();
        let index = self.tail % T::RING_SIZE as usize;
        let limit = (self.mark - self.tail) as usize;
        if len < limit {
            self.write_block_len(src, index, len);
            false
        } else {
            self.write_block_len(src, index, limit);
            true
        }
    }

    #[inline(always)]
    fn write_block_len(&mut self, src: &mut &[u8], index: usize, len: usize) -> Option<()> {
        self.ring.get_mut(index..index + len)?.copy_from_slice(&src[0..len]);
        self.tail += len as u32;
        *src = src.get(len..)?;
        Some(())
    }

    // #[inline(always)]
    fn match_block<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert_eq!(self.tail, self.mark);
        self.manage_ring_zones();
        if self.mark != self.head + T::RING_SIZE {
            self.mark += T::RING_BLK_SIZE;
            return Ok(());
        }
        self.commit(backend, dst, Commit::Fse)?;
        self.match_long(backend, dst)?;
        self.reposition_head();
        self.push_literal_overflow(backend, dst)?;
        self.clamp();
        self.mark = self.tail + T::RING_BLK_SIZE;
        Ok(())
    }

    #[inline(always)]
    fn commit<B, O>(&mut self, backend: &mut B, dst: &mut O, commit: Commit) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert!(self.commit == Commit::None || self.commit == commit);
        if self.commit == Commit::None {
            self.commit = commit;
            backend.init(dst, None)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn reposition_head(&mut self) {
        let delta = (self.idx - self.head) as u32;
        let delta = (delta - T::RING_SIZE / 2) / T::RING_BLK_SIZE * T::RING_BLK_SIZE;
        self.head += delta;
    }

    #[inline(always)]
    fn push_literal_overflow<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        if self.literal_idx < self.head {
            // We have literals that have have passed our buffer head without a match, we'll push
            // them as is.
            // We could push pending, but we don't, we discard it. The compression loss is
            // negligible, at most we lose `GOOD_MATCH - 1` bytes in a situation with a low
            // probability of occurrence. Instead we take the reduction in code complexity/ size.
            self.pending.match_len = 0;
            self.push_literals(backend, dst, (self.head - self.literal_idx) as u32)?;
        }
        Ok(())
    }

    /// Call after init, otherwise behavior is undefined.
    pub fn flush<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        self.validate_global();
        self.manage_ring_zones();
        // Select.
        match self.commit {
            Commit::Fse => {
                self.finalize(backend, dst)?;
            }
            Commit::Vn => {
                panic!("internal error: invalid commit state: {:?}", self.commit);
            }
            Commit::None => self.flush_select(backend, dst)?,
        };
        debug_assert!(self.is_done());
        // Eos.
        dst.write_short_u32(MagicBytes::Eos.into())?;
        Ok(())
    }

    fn flush_select<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        debug_assert!(self.is_uncommitted());
        let len = (self.tail - self.idx) as u32;
        if len > VN_CUTOFF {
            self.commit = Commit::Fse;
            self.flush_backend(backend, dst)
        } else if len > RAW_CUTOFF {
            self.commit = Commit::Vn;
            self.flush_backend(&mut VnBackend::default(), dst)
        } else {
            self.flush_raw(dst)
        }
    }

    fn flush_backend<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        let mark = dst.pos();
        let len = (self.tail - self.idx) as u32;
        backend.init(dst, Some(len))?;
        self.finalize(backend, dst)?;
        let dst_len = (dst.pos() - mark) as u32;
        let src_len = (self.tail - Idx::default()) as u32;
        if dst_len < src_len + RAW_HEADER_SIZE {
            return Ok(());
        }
        dst.truncate(mark);
        self.flush_raw(dst)
    }

    fn flush_raw<O>(&mut self, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        let bytes = self.ring.view(self.head, self.tail);
        raw::raw_compress(dst, bytes)?;
        self.literal_idx = self.tail;
        Ok(())
    }

    fn finalize<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert!(self.tail < self.head + T::RING_SIZE);
        self.match_short(backend, dst)?;
        self.flush_pending(backend, dst)?;
        self.flush_literals(backend, dst)?;
        backend.finalize(dst)?;
        Ok(())
    }

    #[inline(always)]
    fn match_long<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert!(self.is_long());
        let mut idx = self.idx;
        self.idx = self.head + T::RING_SIZE / 2 + T::RING_BLK_SIZE;
        loop {
            // Hot loop.
            let u = self.ring.get_u32(idx);
            let u_idx = UIdx::new(u, idx);
            let queue = self.table.push::<B::Type>(u_idx);
            let incoming = self.find_match::<B::Type, false>(queue, u_idx, Self::MAX_MATCH_LEN);
            if let Some(select) = self.pending.select::<GOOD_MATCH_LEN>(incoming) {
                self.push_match(backend, dst, select)?;
                idx += 1;
                for _ in 0..(self.literal_idx - idx) {
                    let u = self.ring.get_u32(idx);
                    let u_idx = UIdx::new(u, idx);
                    self.table.push::<B::Type>(u_idx);
                    idx += 1;
                }
                if idx >= self.idx {
                    // Unlikely
                    self.idx = idx;
                    break;
                }
            } else {
                idx += 1;
                if idx == self.idx {
                    // Unlikely
                    break;
                }
            }
        }
        debug_assert!(self.is_long_post());
        Ok(())
    }

    #[inline(always)]
    fn match_short<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert!(self.is_short());
        let len = self.tail - self.idx;
        if len < 4 {
            return Ok(());
        }
        let mut idx = self.idx;
        self.idx = self.tail - B::Type::MATCH_UNIT + 1;
        loop {
            // Hot loop.
            let u = self.ring.get_u32(idx);
            let u_idx = UIdx::new(u, idx);
            let queue = self.table.push::<B::Type>(u_idx);
            let max = (self.tail - idx) as u32;
            let incoming = self.find_match::<B::Type, true>(queue, u_idx, max);
            if let Some(select) = self.pending.select::<GOOD_MATCH_LEN>(incoming) {
                self.push_match(backend, dst, select)?;
                if self.literal_idx >= self.idx {
                    // Unlikely.
                    self.idx = self.literal_idx;
                    break;
                }
                idx += 1;
                for _ in 0..(self.literal_idx - idx) {
                    let u = self.ring.get_u32(idx);
                    let u_idx = UIdx::new(u, idx);
                    self.table.push::<B::Type>(u_idx);
                    idx += 1;
                }
                if idx >= self.idx {
                    // Unlikely
                    self.idx = idx;
                    break;
                }
            } else {
                idx += 1;
                if idx == self.idx {
                    // Unlikely
                    break;
                }
            }
        }
        debug_assert!(self.is_short_post());
        Ok(())
    }

    #[inline(always)]
    fn find_match<B, const F: bool>(&self, queue: History, item: UIdx, max: u32) -> Match
    where
        B: BackendType,
    {
        debug_assert!(B::MATCH_UNIT <= max);
        debug_assert!(item.idx + max <= self.tail - if F { 0 } else { OVERMATCH_LEN });
        let mut m = Match::default();
        for &match_idx_val in queue.iter() {
            let distance = (item.idx - match_idx_val.idx) as u32;
            debug_assert!(distance < CLAMP_INTERVAL * 3);
            if distance > B::MAX_MATCH_DISTANCE {
                break;
            }
            let match_len_inc = self.match_unit_coarse::<B>(item, match_idx_val, max);
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
            let literal_len = (item.idx - self.literal_idx) as u32;
            m.idx = item.idx;
            if F {
                m.match_len = m.match_len.min(max);
            }
            let max = ((m.match_idx - self.head) as u32).min(literal_len);
            let match_len_dec = self.match_dec_coarse::<B>(m.idx, m.match_idx, max).min(max);
            m.idx -= match_len_dec;
            m.match_idx -= match_len_dec;
            m.match_len += match_len_dec;
            debug_assert!(self.validate_match::<B>(m));
            m
        }
    }

    #[inline(always)]
    fn match_unit_coarse<M: MatchUnit>(&self, item: UIdx, match_item: UIdx, max: u32) -> u32 {
        debug_assert!(self.validate_match_items::<M>(item, match_item));
        let len = M::match_us((item.u, match_item.u));
        if len == 4 {
            self.ring.match_inc_coarse::<4>((item.idx, match_item.idx), max as usize) as u32
        } else {
            len
        }
    }

    #[inline(always)]
    fn match_dec_coarse<M: MatchUnit>(&self, idx: Idx, match_idx: Idx, literal_len: u32) -> u32 {
        debug_assert!(self.validate_match_idxs::<M>(idx, match_idx));
        self.ring.match_dec_coarse::<0>((idx, match_idx), literal_len as usize) as u32
    }

    #[inline(always)]
    fn flush_pending<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
    ) -> io::Result<()> {
        if self.pending.match_len != 0 {
            self.push_match(backend, dst, self.pending)?;
            self.pending.match_len = 0;
        }
        Ok(())
    }

    #[inline(always)]
    fn push_match<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
        m: Match,
    ) -> io::Result<()> {
        if !self.validate_match::<B::Type>(m) {
            return Err(io::Error::new(ErrorKind::InvalidData, "invalid match"));
        }
        let match_len = m.match_len;
        let match_distance = MatchDistance::new((m.idx - m.match_idx) as u32);
        let literals = self.ring.view(self.literal_idx, m.idx);
        self.literal_idx = m.idx + m.match_len;
        backend.push_match(dst, literals, match_len, match_distance)
    }

    #[inline(always)]
    fn flush_literals<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
    ) -> io::Result<()> {
        let len = (self.tail - self.literal_idx) as u32;
        if len != 0 {
            self.push_literals(backend, dst, len)?;
        }
        Ok(())
    }

    fn push_literals<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
        len: u32,
    ) -> io::Result<()> {
        debug_assert_ne!(len, 0);
        debug_assert_eq!(self.pending.match_len, 0);
        debug_assert!(self.literal_idx + len <= self.tail);
        let literals = self.ring.view(self.literal_idx, self.literal_idx + len);
        self.literal_idx += len;
        backend.push_literals(dst, literals)
    }

    pub fn init(&mut self) {
        let zero = Idx::default();
        self.table.reset_idx(zero - CLAMP_INTERVAL);
        self.commit = Commit::None;
        self.pending = Match::default();
        self.head = zero;
        self.literal_idx = zero;
        self.idx = zero;
        self.tail = zero;
        self.mark = zero + T::RING_BLK_SIZE;
        self.clamp = zero + CLAMP_INTERVAL;
        debug_assert!(self.is_init());
    }

    fn manage_ring_zones(&mut self) {
        if self.mark % T::RING_SIZE == T::RING_BLK_SIZE {
            self.ring.head_copy_out();
        } else if self.mark % T::RING_SIZE == 0 {
            self.ring.tail_copy_out();
        }
    }

    fn clamp(&mut self) {
        let delta = self.idx - self.clamp;
        debug_assert!(delta < CLAMP_INTERVAL as i32 && delta >= -(CLAMP_INTERVAL as i32));
        if delta >= 0 {
            // Unlikely.
            assert!(delta < CLAMP_INTERVAL as i32);
            self.table.clamp(self.idx);
            self.clamp += CLAMP_INTERVAL;
        }
    }

    fn is_init(&self) -> bool {
        let zero = Idx::default();
        self.is_uncommitted()
            && self.tail == zero
            && self.mark == zero + T::RING_BLK_SIZE
            && self.n_raw_bytes == 0
    }

    fn is_uncommitted(&self) -> bool {
        let zero = Idx::default();
        self.commit == Commit::None
            && self.pending == Match::default()
            && self.head == zero
            && self.literal_idx == zero
            && self.idx == zero
    }

    fn is_long(&self) -> bool {
        self.validate_global()
            && (self.head == self.idx || self.head + T::RING_SIZE / 2 <= self.idx)
            && self.idx < self.head + T::RING_SIZE / 2 + T::RING_BLK_SIZE
            && self.tail == self.mark
            && self.mark == self.head + T::RING_SIZE
    }

    fn is_long_post(&self) -> bool {
        self.validate_global()
            && self.head + T::RING_SIZE / 2 + T::RING_BLK_SIZE <= self.idx
            && self.tail == self.mark
            && self.mark == self.head + T::RING_SIZE
    }

    fn is_short(&self) -> bool {
        self.validate_global() && self.tail < self.head + T::RING_SIZE
    }

    fn is_short_post(&self) -> bool {
        self.validate_global() && self.tail - 4 <= self.idx
    }

    fn is_done(&self) -> bool {
        self.validate_clamp() && self.literal_idx == self.tail
    }

    fn validate_clamp(&self) -> bool {
        let delta = self.idx - self.clamp;
        -(CLAMP_INTERVAL as i32) <= delta && delta < CLAMP_INTERVAL as i32 / 2
    }

    fn validate_global(&self) -> bool {
        self.head <= self.literal_idx
            && self.literal_idx <= self.idx
            && self.idx <= self.tail
            && self.tail <= self.mark
            && self.mark <= self.head + T::RING_SIZE
            && self.mark % T::RING_BLK_SIZE == 0
    }

    fn validate_match<B: BackendType>(&self, m: Match) -> bool {
        m.match_len != 0
            && m.match_len >= B::MATCH_UNIT
            && self.literal_idx <= m.idx
            && m.match_idx < m.idx
            && (m.idx - m.match_idx) as u32 <= B::MAX_MATCH_DISTANCE
            && m.idx + m.match_len <= self.tail
    }

    fn validate_match_items<M: MatchUnit>(&self, item: UIdx, match_item: UIdx) -> bool {
        self.validate_match_idxs::<M>(item.idx, match_item.idx)
            && (item.u ^ self.ring.get_u32(item.idx)) & M::MATCH_MASK == 0
            && (match_item.u ^ self.ring.get_u32(match_item.idx)) & M::MATCH_MASK == 0
    }

    fn validate_match_idxs<M: MatchUnit>(&self, idx: Idx, match_idx: Idx) -> bool {
        self.head <= match_idx && match_idx < idx && idx <= self.tail - M::MATCH_UNIT
    }
}

impl<'a, T: RingType> AsMut<[u8]> for FrontendRing<'a, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.ring
    }
}

#[cfg(test)]
mod tests {
    use crate::lmd::{LiteralLen, Lmd};
    use crate::ring::RingBox;
    use crate::ring::RingSize;

    use super::super::dummy::{Dummy, DummyBackend};
    use super::*;

    use std::io;

    #[derive(Copy, Clone, Debug)]
    pub struct T;

    impl RingSize for T {
        const RING_SIZE: u32 = 0x0001_0000;
    }

    impl RingType for T {
        const RING_LIMIT: u32 = 0x0100;
    }

    impl RingBlock for T {
        const RING_BLK_SIZE: u32 = 0x0200;
    }

    fn build<'a, T>(ring: Ring<'a, T>, table: &'a mut HistoryTable) -> FrontendRing<'a, T>
    where
        T: Copy + RingBlock,
    {
        assert!(T::RING_BLK_SIZE.is_power_of_two());
        assert!(T::RING_SIZE <= CLAMP_INTERVAL / 4);
        assert!(0x100 < T::RING_BLK_SIZE as usize);
        assert!(T::RING_BLK_SIZE <= T::RING_SIZE / 4);
        assert!(OVERMATCH_LEN < T::RING_LIMIT);
        let zero = Idx::default();
        FrontendRing {
            table,
            ring,
            pending: Match::default(),
            literal_idx: zero,
            idx: zero,
            head: zero,
            mark: zero,
            tail: zero,
            clamp: zero,
            commit: Commit::None,
            n_raw_bytes: 0,
        }
    }

    // Match short: zero bytes, length 4. Lower limit.
    #[test]
    fn match_short_zero_4() -> io::Result<()> {
        let mut ring_box = RingBox::<T>::default();
        let mut table = HistoryTable::default();
        let mut frontend = build((&mut ring_box).into(), &mut table);
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        let base = Idx::default();
        frontend.table.reset_idx(base - CLAMP_INTERVAL);
        frontend.pending = Match::default();
        frontend.literal_idx = base;
        frontend.idx = base;
        frontend.head = base;
        frontend.tail = base + 4;
        frontend.mark = base + T::RING_BLK_SIZE;
        frontend.match_short(&mut backend, &mut dst).unwrap();
        if frontend.pending.match_len != 0 {
            unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
        }
        let literal_len = (frontend.tail - frontend.literal_idx) as u32;
        if literal_len > 0 {
            frontend.push_literals(&mut backend, &mut dst, literal_len)?;
        }
        assert_eq!(backend.literals, [0]);
        assert_eq!(backend.lmds, vec![Lmd::<Dummy>::new(1, 3, 1)]);
        Ok(())
    }

    // Match short: zero bytes, length 5++.
    #[test]
    #[ignore = "expensive"]
    fn match_short_zero_n() -> io::Result<()> {
        let mut ring_box = RingBox::<T>::default();
        let mut table = HistoryTable::default();
        let mut frontend = build((&mut ring_box).into(), &mut table);
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        let base = Idx::default();
        for n in 5..T::RING_SIZE {
            backend.init(&mut dst, None)?;
            frontend.table.reset_idx(base - CLAMP_INTERVAL);
            frontend.pending = Match::default();
            frontend.literal_idx = base;
            frontend.idx = base;
            frontend.head = base;
            frontend.tail = base + n;
            frontend.mark =
                base + ((n + T::RING_BLK_SIZE - 1) / T::RING_BLK_SIZE) * T::RING_BLK_SIZE;
            frontend.match_short(&mut backend, &mut dst)?;
            if frontend.pending.match_len != 0 {
                unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
            }
            assert_eq!(frontend.literal_idx, frontend.tail);
            assert_eq!(backend.literals, [0]);
            assert_eq!(backend.lmds, vec![Lmd::<Dummy>::new(1, n - 1, 1)]);
        }
        Ok(())
    }

    // Match long, zero bytes, check that overmatch limit doesn't breach tail.
    #[test]
    #[ignore = "expensive"]
    fn match_long_overmatch_limit() -> io::Result<()> {
        let mut ring_box = RingBox::<T>::default();
        let mut table = HistoryTable::default();
        let mut frontend = build((&mut ring_box).into(), &mut table);
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        for offset in 0..T::RING_BLK_SIZE - 1 {
            backend.init(&mut dst, None)?;
            let base = Idx::default();
            let idx = base + T::RING_SIZE / 2 + offset;
            frontend.table.reset_idx(base - CLAMP_INTERVAL);
            frontend.pending = Match::default();
            frontend.literal_idx = idx;
            frontend.idx = idx;
            frontend.head = base;
            frontend.tail = base + T::RING_SIZE;
            frontend.mark = base + T::RING_SIZE;
            frontend.match_long(&mut backend, &mut dst)?;
            if frontend.pending.match_len != 0 {
                unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
            }
            assert_eq!(backend.literals, []);
            assert_eq!(backend.lmds.len(), 1);
            assert_eq!(backend.lmds[0].0, LiteralLen::new(0));
            assert!(idx + backend.lmds[0].1.get() <= frontend.tail);
            assert_eq!(backend.lmds[0].2.get(), 1);
            assert!(dst.is_empty());
        }
        Ok(())
    }

    // Sandwich, incremental literals.
    #[test]
    #[ignore = "expensive"]
    fn sandwich_n_short() -> io::Result<()> {
        let mut ring_box = RingBox::<T>::default();
        let mut table = HistoryTable::default();
        let mut frontend = build((&mut ring_box).into(), &mut table);
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        for i in 0..3 {
            frontend.ring[i] = i as u8 + 1;
        }
        let base = Idx::default();
        for n in 10..T::RING_SIZE {
            backend.init(&mut dst, None)?;
            for i in 0..3 {
                frontend.ring[n as usize - 3 + i] = i as u8 + 1;
            }
            frontend.ring.head_copy_out();
            frontend.ring.tail_copy_out();
            frontend.table.reset_idx(base - CLAMP_INTERVAL);
            frontend.pending = Match::default();
            frontend.literal_idx = base;
            frontend.idx = base;
            frontend.head = base;
            frontend.tail = base + n;
            frontend.mark =
                base + ((n + T::RING_BLK_SIZE - 1) / T::RING_BLK_SIZE) * T::RING_BLK_SIZE;
            frontend.match_short(&mut backend, &mut dst)?;
            if frontend.pending.match_len != 0 {
                unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
            }
            assert_eq!(frontend.literal_idx, frontend.tail);
            assert_eq!(backend.literals, [1, 2, 3, 0]);
            assert_eq!(
                backend.lmds,
                vec![Lmd::<Dummy>::new(4, n - 7, 1), Lmd::<Dummy>::new(0, 3, n - 3),]
            );
            frontend.ring[n as usize - 3] = 0;
        }
        Ok(())
    }
}
