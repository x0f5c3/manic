mod backend;
mod backend_type;
mod constants;
mod encoder;
mod frontend_bytes;
mod frontend_ring;
mod history;
mod match_object;
mod match_unit;
mod ring_encoder;
mod writer;
mod writer_bytes;

#[cfg(test)]
mod dummy;

pub use backend::Backend;
pub use backend_type::BackendType;
pub use encoder::LzfseEncoder;
pub use match_unit::MatchUnit;
pub use ring_encoder::LzfseRingEncoder;
pub use writer::LzfseWriter;
pub use writer_bytes::LzfseWriterBytes;

use std::io;

/// Encode `src` into `dst` returning the number of bytes written into `dst`.
///
/// Due to internal mechanics `src` is cannot exceed `i32::MAX` bytes in length.
///
/// This is a convenience method that constructs a temporary [LzfseEncoder] instance and then calls
/// [encode_bytes](LzfseEncoder::encode_bytes). For multiple invocations, creating and reusing a
/// [LzfseEncoder] instance is more efficient.
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
/// use std::io;
///
/// fn main() -> io::Result<()> {
///     let mut enc = Vec::default();
///     let n_bytes = manic_lzfse::encode_bytes(b"test", &mut enc)?;
///     assert_eq!(n_bytes, 16);
///     // "test" string encoded.
///     assert_eq!(enc, &[0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x74, 0x65, 0x73, 0x74,
///                       0x62, 0x76, 0x78, 0x24]);
///     Ok(())
/// }
/// ```
pub fn encode_bytes(src: &[u8], dst: &mut Vec<u8>) -> io::Result<u64> {
    LzfseEncoder::default().encode_bytes(src, dst)
}
