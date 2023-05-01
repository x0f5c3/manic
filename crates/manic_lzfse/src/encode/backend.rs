use crate::lmd::MatchDistance;
use crate::types::{ShortBuffer, ShortWriter};

use super::backend_type::BackendType;

use std::io;

pub trait Backend {
    type Type: BackendType;

    /// Initialize the backend.
    ///
    /// `len` is the input length with None indicating that either the exact value is unknown or
    /// is larger than `u32::MAX`.
    fn init<O: ShortWriter>(&mut self, dst: &mut O, len: Option<u32>) -> io::Result<()>;

    /// Push a literal block. Literals should be coalesced into the largest possible block size,
    /// failure to do so will reduce the compression ratio or may cause the backend to panic.
    fn push_literals<I: ShortBuffer, O: ShortWriter>(
        &mut self,
        dst: &mut O,
        literals: I,
    ) -> io::Result<()>;

    fn push_match<I: ShortBuffer, O: ShortWriter>(
        &mut self,
        dst: &mut O,
        literals: I,
        match_len: u32,
        match_distance: MatchDistance<Self::Type>,
    ) -> io::Result<()>;

    /// Implementations should NOT flush `dst`.
    fn finalize<O: ShortWriter>(&mut self, dst: &mut O) -> io::Result<()>;
}
