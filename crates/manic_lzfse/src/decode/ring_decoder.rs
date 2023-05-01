use crate::ring::{RingBox, RingLzWriter, RingReader};

use super::constants::*;
use super::decoder::LzfseDecoder;
use super::reader_core::ReaderCore;

use std::fmt;
use std::io::{self, Read, Write};

/// LZFSE ring decoder.
///
///
/// This implementation builds upon [LzfseDecoder] with the addition of internal ring buffers that
/// enable efficient IO operations. It can be converted to a mutable [LzfseDecoder] reference using
/// [as_mut()](AsMut::as_mut).
pub struct LzfseRingDecoder {
    core: LzfseDecoder,
    input: RingBox<Input>,
    output: RingBox<Output>,
}

impl LzfseRingDecoder {
    /// Decode `reader` into `writer` returning a tuple (u, v) where u is the number of encoded
    /// bytes read from the reader and v is the number of decoded bytes written into the writer.
    ///
    /// Both the `reader` and `writer` are accessed efficiently by internal ring buffers, there is
    /// no need to wrap them in [BufReader](std::io::BufReader) or
    /// [BufWriter](std::io::BufWriter).
    ///
    /// # Errors
    ///
    /// * [Error](crate::Error) detailing the nature of any errors.
    ///
    /// # Examples
    ///
    /// ```
    /// use manic_lzfse::LzfseRingDecoder;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     // "test" string encoded.
    ///     let enc = vec![
    ///         0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74, 0x62, 0x76,
    ///         0x78, 0x24,
    ///     ];
    ///     let mut decoder = LzfseRingDecoder::default();
    ///     let mut reader = enc.as_slice();
    ///     let mut writer = Vec::default();
    ///     let (u, v) = decoder.decode(&mut reader, &mut writer)?;
    ///     assert_eq!(u, 16);
    ///     assert_eq!(v, 4);
    ///     assert_eq!(writer, b"test");
    ///     Ok(())
    /// }
    /// ```
    pub fn decode<I: Read, O: Write>(
        &mut self,
        reader: &mut I,
        writer: &mut O,
    ) -> crate::Result<(u64, u64)> {
        let mut dst = RingLzWriter::new((&mut self.output).into(), writer);
        let mut src = RingReader::new((&mut self.input).into(), reader);
        let n = self.core.execute(&mut dst, &mut src)?;
        dst.into_inner()?;
        Ok(n)
    }

    /// This method bypasses the internal ring buffers and operates over the supplied buffers,
    /// it is functionally identical to [LzfseDecoder::decode_bytes].
    pub fn decode_bytes(&mut self, src: &[u8], dst: &mut Vec<u8>) -> crate::Result<u64> {
        self.core.decode_bytes(src, dst)
    }

    /// Create a new [LzfseReader] decoder instance using the supplied `inner` reader.
    pub fn reader<I: Read>(&mut self, inner: I) -> LzfseReader<I> {
        let dst = RingLzWriter::new((&mut self.output).into(), io::sink());
        let src = RingReader::new((&mut self.input).into(), inner);
        LzfseReader(ReaderCore::new(dst, src, &mut self.core.fse_core))
    }

    /// Create a new [LzfseReaderBytes] decoder instance using the supplied `bytes`.
    ///
    /// This method offers greater efficiency in comparison to [LzfseRingDecoder::reader]
    /// when operating over byte slices.
    pub fn reader_bytes<'a>(&'a mut self, bytes: &'a [u8]) -> LzfseReaderBytes {
        let dst = RingLzWriter::new((&mut self.output).into(), io::sink());
        LzfseReaderBytes(ReaderCore::new(dst, bytes, &mut self.core.fse_core))
    }
}

impl Default for LzfseRingDecoder {
    fn default() -> Self {
        Self {
            core: LzfseDecoder::default(),
            input: RingBox::default(),
            output: RingBox::default(),
        }
    }
}

impl AsMut<LzfseDecoder> for LzfseRingDecoder {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut LzfseDecoder {
        &mut self.core
    }
}

impl fmt::Debug for LzfseRingDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LzfseRingDecoder").finish()
    }
}

/// LZFSE decoding reader.
///
/// Exposes a LZFSE decoder via the [Read](std::io::Read) interface that decodes from
/// an inner reader.
///
/// Instances are created using
/// [LzfseRingDecoder::reader](super::LzfseRingDecoder::reader).
///
/// # Examples
///
/// ```
/// use manic_lzfse::LzfseRingDecoder;
/// use std::io::{self, Read};
///
/// fn main() -> io::Result<()> {
///     // "test" string encoded.
///     let enc = vec![
///         0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74, 0x62, 0x76,
///         0x78, 0x24,
///     ];
///     let mut decoder = LzfseRingDecoder::default();
///     let inner = enc.as_slice();
///     let mut reader = decoder.reader(inner);
///     let mut dec = Vec::default();
///     reader.read_to_end(&mut dec)?;
///     assert_eq!(dec, b"test");
///     Ok(())
/// }
/// ```

pub struct LzfseReader<'a, I: Read>(ReaderCore<'a, RingReader<'a, I, Input>>);

impl<'a, I: Read> LzfseReader<'a, I> {
    /// Unwraps and returns the underlying reader.
    ///
    /// Note that unless all data has been read, in which case the underlying reader also has
    /// been fully read, the position of underlying reader in undefined.
    pub fn into_inner(self) -> I {
        self.0.into_inner().into_inner()
    }
}

impl<'a, I: Read> fmt::Debug for LzfseReader<'a, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LzfseReader").finish()
    }
}

impl<'a, I: Read> Read for LzfseReader<'a, I> {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

/// LZFSE decoding byte reader.
///
/// Exposes a LZFSE decoder via the [Read](std::io::Read) interface that decodes from
/// an inner [Vec].
///
/// Instances are created using
/// [LzfseRingDecoder::reader_bytes](super::LzfseRingDecoder::reader_bytes).
///
/// # Examples
///
/// ```
/// use manic_lzfse::LzfseRingDecoder;
/// use std::io::{self, Read};
///
/// fn main() -> io::Result<()> {
///     // "test" string encoded.
///     let enc = vec![
///         0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74, 0x62, 0x76,
///         0x78, 0x24,
///     ];
///     let mut decoder = LzfseRingDecoder::default();
///     let mut reader = decoder.reader_bytes(&enc);
///     let mut dec = Vec::default();
///     reader.read_to_end(&mut dec)?;
///     assert_eq!(dec, b"test");
///     Ok(())
/// }
/// ```
pub struct LzfseReaderBytes<'a>(ReaderCore<'a, &'a [u8]>);

impl<'a> Read for LzfseReaderBytes<'a> {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl<'a> fmt::Debug for LzfseReaderBytes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LzfseReaderBytes").finish()
    }
}
