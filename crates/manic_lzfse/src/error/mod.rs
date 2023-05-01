use crate::fse;
use crate::vn;

use std::error;
use std::fmt;
use std::io;

/// Decoding error Result.
pub type Result<T> = std::result::Result<T, Error>;

/// Decoding errors.
///
/// You may want to convert [Error] to [io::Error](std::io::Error) either directly or by using
/// the `?` operator, see the examples below. Reporting information is preserved across the
/// conversion. [Error::Io] errors are flattened whilst other errors are boxed into an
/// [InvalidData](std::io::ErrorKind::InvalidData) variant [io::Error](std::io::Error).
///
/// # Examples
///
/// ```
/// use std::io;
///
/// // Direct conversion
/// pub fn decode(src: &[u8], dst: &mut Vec<u8>) -> io::Result<u64> {
///     manic_lzfse::decode_bytes(src, dst).map_err(Into::into)
/// }
/// ```
///
/// ```
/// use std::io;
///
/// // `?` operator conversion
/// pub fn decode(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
///     manic_lzfse::decode_bytes(src, dst)?;
///     Ok(())
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// [IO](std::io) errors.
    Io(io::Error),
    /// FSE specific errors.
    Fse(fse::FseErrorKind),
    /// VN specific errors.
    Vn(vn::VnErrorKind),
    /// Unknown or unsupported block type (magic bytes).
    BadBlock(u32),
    /// Bad bitstream.
    BadBitStream,
    /// Bad LZ distance value.
    BadDValue,
    /// Reader state is invalid, likely the user attempted to use after an error was encountered.
    BadReaderState,
    /// Buffer overflow.
    BufferOverflow,
    /// Input has more bytes than expected.
    PayloadOverflow,
    /// Input has less bytes than expected.
    PayloadUnderflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        match self {
            Self::Io(e) => write!(f, "IO: {}", e),
            Self::Fse(e) => write!(f, "FSE: {}", e),
            Self::Vn(e) => write!(f, "VN: {}", e),
            Self::BadBitStream => write!(f, "bad bitstream"),
            Self::BadDValue => write!(f, "bad D value"),
            Self::BadBlock(u) => write!(f, "bad block: 0x{:08X}", u),
            Self::BadReaderState => write!(f, "bad reader state"),
            Self::BufferOverflow => write!(f, "buffer overflow"),
            Self::PayloadOverflow => write!(f, "bad payload overflow"),
            Self::PayloadUnderflow => write!(f, "bad payload underflow"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<fse::FseErrorKind> for Error {
    fn from(err: fse::FseErrorKind) -> Error {
        Error::Fse(err)
    }
}

impl From<vn::VnErrorKind> for Error {
    fn from(err: vn::VnErrorKind) -> Error {
        Error::Vn(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Io(e) => e,
            err => io::Error::new(io::ErrorKind::InvalidData, err),
        }
    }
}
