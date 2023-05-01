use crate::fse::{V1_MAX_BLOCK_LEN, V2_MAX_BLOCK_LEN};
use crate::ops::FlushLimit;
use crate::ring::{RingBox, RingShortWriter};

use super::constants::*;
use super::encoder::LzfseEncoder;
use super::frontend_ring::FrontendRing;
use super::writer::LzfseWriter;
use super::writer_bytes::LzfseWriterBytes;

use std::fmt;
use std::io::{self, Read, Write};

/// LZFSE ring encoder.
///
///
/// This implementation builds upon [LzfseEncoder] with the addition of internal ring buffers that
/// enable efficient IO operations. It can be converted to a mutable [LzfseEncoder] reference using
/// [as_mut()](AsMut::as_mut).
pub struct LzfseRingEncoder {
    core: LzfseEncoder,
    input: RingBox<Input>,
    output: RingBox<Output>,
}

impl LzfseRingEncoder {
    /// Encode `reader` into `writer` returning a tuple (u, v) where u is the number of unencoded
    /// bytes read from the reader and v is the number of encoded bytes written into the writer.
    ///
    /// Both the `reader` and `writer` are accessed efficiently by internal ring buffers, there is
    /// no need to wrap them in [BufReader](std::io::BufReader) or
    /// [BufWriter](std::io::BufWriter).
    ///
    ///
    /// # Errors
    ///
    /// * [Error](std::io::Error) in case of `reader` or `writer` IO errors.
    ///
    /// # Examples
    ///
    /// ```
    /// use manic_lzfse::LzfseRingEncoder;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut enc = Vec::default();
    ///     let mut encoder = LzfseRingEncoder::default();
    ///     let n_bytes = encoder.encode_bytes(b"test", &mut enc)?;
    ///     // "test" string encoded.
    ///     assert_eq!(enc, &[0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74,
    ///                       0x62, 0x76, 0x78, 0x24]);
    ///     Ok(())
    /// }
    /// ```
    pub fn encode<I, O>(&mut self, reader: &mut I, writer: &mut O) -> io::Result<(u64, u64)>
    where
        I: Read,
        O: Write,
    {
        let mut frontend = FrontendRing::new((&mut self.input).into(), &mut self.core.table);
        frontend.init();
        let mut writer = RingShortWriter::new((&mut self.output).into(), writer);
        let n_raw_bytes = frontend.copy(&mut self.core.backend, &mut writer, reader)?;
        frontend.flush(&mut self.core.backend, &mut writer)?;
        let (_, n_payload_bytes) = writer.into_inner()?;
        Ok((n_raw_bytes, n_payload_bytes))
    }

    /// This method bypasses the internal ring buffers and operates over the supplied buffers,
    /// it is functionally identical to [LzfseEncoder::encode_bytes].
    pub fn encode_bytes(&mut self, src: &[u8], dst: &mut Vec<u8>) -> io::Result<u64> {
        self.core.encode_bytes(src, dst)
    }

    /// Create a new [LzfseWriter] encoder instance using the supplied `inner` writer.
    ///
    /// **It is imperative that the writer is [finalized](LzfseWriterBytes::finalize) after use to
    /// complete the encoding process, [flushing](std::io::Write::flush) is not sufficient.**
    pub fn writer<O: Write>(&mut self, inner: O) -> LzfseWriter<O> {
        let mut frontend = FrontendRing::new((&mut self.input).into(), &mut self.core.table);
        frontend.init();
        let writer = RingShortWriter::new((&mut self.output).into(), inner);
        LzfseWriter::new(frontend, &mut self.core.backend, writer)
    }

    /// Create a new [LzfseWriterBytes] decoder instance using the supplied `vec`.
    ///
    /// This method offers greater efficiency in comparison to [LzfseRingEncoder::writer]
    /// when operating over byte vectors.
    ///
    /// **It is imperative that the writer is [finalized](LzfseWriterBytes::finalize) after use to
    /// complete the encoding process, [flushing](std::io::Write::flush) is not sufficient.**
    pub fn writer_bytes(&mut self, vec: Vec<u8>) -> LzfseWriterBytes {
        let mut frontend = FrontendRing::new((&mut self.input).into(), &mut self.core.table);
        frontend.init();
        LzfseWriterBytes::new(frontend, &mut self.core.backend, vec)
    }
}

impl Default for LzfseRingEncoder {
    #[allow(clippy::clippy::assertions_on_constants)]
    fn default() -> Self {
        assert!(V1_MAX_BLOCK_LEN + 64 < RingShortWriter::<(), Output>::FLUSH_LIMIT);
        assert!(V2_MAX_BLOCK_LEN + 64 < RingShortWriter::<(), Output>::FLUSH_LIMIT);
        Self {
            core: LzfseEncoder::default(),
            input: RingBox::<Input>::default(),
            output: RingBox::<Output>::default(),
        }
    }
}

impl fmt::Debug for LzfseRingEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LzfseRingEncoder").finish()
    }
}
