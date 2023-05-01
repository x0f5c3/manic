mod constants;
mod decoder;
mod probe;
mod reader_core;
mod ring_decoder;
mod take;

pub use decoder::LzfseDecoder;
pub use probe::probe;
pub use reader_core::ReaderCore;
pub use ring_decoder::{LzfseReader, LzfseReaderBytes, LzfseRingDecoder};
pub use take::Take;

/// Decode `src` into `dst` returning the number of bytes written into `dst`.
///
///
/// This is a convenience method that constructs a temporary [LzfseDecoder] instance and then calls
/// [decode_bytes](LzfseDecoder::decode_bytes). For multiple invocations, creating and reusing a
/// [LzfseDecoder] instance is more efficient.
///
/// # Errors
///
/// * [Error](crate::Error) detailing the nature of any errors.
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
/// use std::io;
///
/// fn main() -> io::Result<()> {
///     // "test" string encoded.
///     let enc = vec![
///         0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74, 0x62, 0x76,
///         0x78, 0x24,
///     ];
///     let mut dec = Vec::default();
///     let n_bytes = manic_lzfse::decode_bytes(&enc, &mut dec)?;
///     assert_eq!(n_bytes, 4);
///     assert_eq!(dec, b"test");
///     Ok(())
/// }
/// ```
pub fn decode_bytes(src: &[u8], dst: &mut Vec<u8>) -> crate::Result<u64> {
    LzfseDecoder::default().decode_bytes(src, dst)
}
