use std::fmt;

/// VN error kinds.
///
/// Low-level error kinds.
/// As a general rule, these should not be matched against.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum VnErrorKind {
    /// Bad payload count.
    BadPayloadCount(u32),
    /// Bad payload.
    BadPayload,
    /// Bad opcode.
    BadOpcode,
}

impl fmt::Display for VnErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        match self {
            Self::BadPayloadCount(u) => write!(f, "bad payload count: 0x{:08X}", u),
            Self::BadPayload => write!(f, "bad payload"),
            Self::BadOpcode => write!(f, "bad opcode"),
        }
    }
}
