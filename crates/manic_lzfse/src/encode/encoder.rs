use crate::fse::FseBackend;

use super::frontend_bytes::FrontendBytes;
use super::history::HistoryTable;

use std::fmt;
use std::io;

/// LZFSE encoder.
///
///
/// This basic implementation encodes byte slices into byte vectors.
pub struct LzfseEncoder {
    pub(super) backend: FseBackend,
    pub(super) table: HistoryTable,
    dst_mark: u64,
}

impl LzfseEncoder {
    /// Encode `src` into `dst` returning the number of bytes written into `dst`.
    ///
    /// Due to internal mechanics `src` is cannot exceed `i32::MAX` bytes in length.
    ///
    /// # Errors
    ///
    /// * [ErrorKind::Other](std::io::ErrorKind) in case of `src` or `dst` buffer overflow.
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
    /// use manic_lzfse::LzfseEncoder;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut enc = Vec::default();
    ///     let mut encoder = LzfseEncoder::default();
    ///     let n_bytes = encoder.encode_bytes(b"test", &mut enc)?;
    ///     // "test" string encoded.
    ///     assert_eq!(enc, &[0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74,
    ///                       0x62, 0x76, 0x78, 0x24]);
    ///     Ok(())
    /// }
    /// ```
    pub fn encode_bytes(&mut self, src: &[u8], dst: &mut Vec<u8>) -> io::Result<u64> {
        self.dst_mark = dst.len() as u64;
        FrontendBytes::new(&mut self.table, src)?.execute(&mut self.backend, dst)?;
        Ok(dst.len() as u64 - self.dst_mark)
    }
}

impl Default for LzfseEncoder {
    fn default() -> Self {
        Self { backend: FseBackend::default(), table: HistoryTable::default(), dst_mark: 0 }
    }
}

impl fmt::Debug for LzfseEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LzfseEncoder").finish()
    }
}
