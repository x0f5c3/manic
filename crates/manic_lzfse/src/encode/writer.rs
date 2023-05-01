use crate::fse::FseBackend;
use crate::ring::RingShortWriter;

use super::constants::*;
use super::frontend_ring::FrontendRing;

use std::fmt;
use std::io::{self, Write};

/// LZFSE encoding writer.
///
/// Exposes, in part, a LZFSE encoder via the [Write](std::io::Write) interface that encodes into
/// an inner writer. Due to the nature of LZFSE streams [Write::flush](std::io::Write::flush) has
/// no effect, instead it is imperative that we call [LzfseWriter::finalize] after use to complete
/// the encoding process.
///
/// Instances are created using
/// [LzfseRingEncoder::writer_bytes](super::LzfseRingEncoder::writer).
///
/// # Examples
///
/// ```
/// use manic_lzfse::LzfseRingEncoder;
/// use std::io::{self, Write};
///
/// fn main() -> io::Result<()> {
///     let mut encoder = LzfseRingEncoder::default();
///     let inner = Vec::default();
///     let mut writer = encoder.writer(inner);
///     writer.write_all(b"test")?;
///     // It is IMPERATIVE that the writer is finalized.
///     let enc = writer.finalize()?;
///     // "test" string encoded.
///     assert_eq!(enc, &[0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74,
///                       0x62, 0x76, 0x78, 0x24]);
///     Ok(())
/// }
/// ```
pub struct LzfseWriter<'a, O> {
    frontend: FrontendRing<'a, Input>,
    backend: &'a mut FseBackend,
    writer: RingShortWriter<'a, O, Output>,
}

impl<'a, O> LzfseWriter<'a, O> {
    #[inline(always)]
    pub(super) fn new(
        frontend: FrontendRing<'a, Input>,
        backend: &'a mut FseBackend,
        writer: RingShortWriter<'a, O, Output>,
    ) -> Self {
        Self { frontend, backend, writer }
    }
}

impl<'a, O: Write> LzfseWriter<'a, O> {
    /// Finalize the encoding process.
    /// Failure to finalize will likely result in a truncated output.
    pub fn finalize(mut self) -> io::Result<O> {
        self.frontend.flush(&mut self.backend, &mut self.writer)?;
        self.writer.into_inner().map_err(Into::into).map(|u| u.0)
    }
}

impl<'a, O: Write> Write for LzfseWriter<'a, O> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.frontend.write(&mut self.backend, buf, &mut self.writer)
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a, O> fmt::Debug for LzfseWriter<'a, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LzfseWriter").finish()
    }
}
