use crate::fse::FseBackend;

use super::constants::*;
use super::frontend_ring::FrontendRing;

use std::fmt;
use std::io::{self, Write};

/// LZFSE encoding byte writer.
///
/// Exposes, in part, a LZFSE encoder via the [Write](std::io::Write) interface that encodes into
/// an inner [Vec]. Due to the nature of LZFSE streams [Write::flush](std::io::Write::flush) has
/// no effect, instead it is imperative that we call [LzfseWriterBytes::finalize] after use to
/// complete the encoding process.
///
/// Instances are created using
/// [LzfseRingEncoder::writer_bytes](super::LzfseRingEncoder::writer_bytes).
///
/// # Aborts
///
/// With limited system memory [Vec] may abort when attempting to allocate sufficient memory.
/// This issue will be resolved in future releases when [try_reserve()](Vec::try_reserve) is
/// stabilized.
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
///     let mut writer = encoder.writer_bytes(inner);
///     writer.write_all(b"test")?;
///     // It is IMPERATIVE that the writer is finalized.
///     let enc = writer.finalize()?;
///     // "test" string encoded.
///     assert_eq!(enc, &[0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74,
///                       0x62, 0x76, 0x78, 0x24]);
///     Ok(())
/// }
/// ```
pub struct LzfseWriterBytes<'a> {
    frontend: FrontendRing<'a, Input>,
    backend: &'a mut FseBackend,
    vec: Vec<u8>,
}

impl<'a> LzfseWriterBytes<'a> {
    #[inline(always)]
    pub(super) fn new(
        frontend: FrontendRing<'a, Input>,
        backend: &'a mut FseBackend,
        vec: Vec<u8>,
    ) -> Self {
        Self { frontend, backend, vec }
    }

    /// Finalize the encoding process.
    /// Failure to finalize will likely result in a truncated output.
    pub fn finalize(mut self) -> io::Result<Vec<u8>> {
        self.frontend.flush(&mut self.backend, &mut self.vec)?;
        Ok(self.vec)
    }
}

impl<'a> Write for LzfseWriterBytes<'a> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.frontend.write(&mut self.backend, buf, &mut self.vec)
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> fmt::Debug for LzfseWriterBytes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LzfseWriterBytes").finish()
    }
}
