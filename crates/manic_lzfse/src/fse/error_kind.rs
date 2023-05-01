use std::fmt;

/// FSE error kinds.
///
/// Low-level error kinds.
/// As a general rule, these should not be matched against.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FseErrorKind {
    /// Bad literal bits.
    BadLiteralBits,
    /// Bad literal count.
    BadLiteralCount(u32),
    /// Bad literal payload.
    BadLiteralPayload,
    /// Bad literal state.
    BadLiteralState,
    /// Bad LMD bits.
    BadLmdBits,
    /// Bad LMD count.
    BadLmdCount(u32),
    /// Bad LMD payload.
    BadLmdPayload,
    /// Bad LMD state.
    BadLmdState,
    /// Bad payload count.
    BadPayloadCount,
    /// Bad raw byte count.
    BadRawByteCount,
    /// Bad reader state.
    BadReaderState,
    /// Bad weight payload.
    BadWeightPayload,
    /// Bad weight payload count.
    BadWeightPayloadCount,
    /// Weight payload overflow.    
    WeightPayloadOverflow,
    /// Weight payload underflow.
    WeightPayloadUnderflow,
}

impl fmt::Display for FseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        match self {
            Self::BadLiteralBits => write!(f, "bad literal bits"),
            Self::BadLiteralCount(u) => write!(f, "bad literal count: 0x{:08X}", u),
            Self::BadLiteralPayload => write!(f, "bad literal payload"),
            Self::BadLiteralState => write!(f, "bad literal state"),
            Self::BadLmdBits => write!(f, "bad lmd bits"),
            Self::BadLmdCount(u) => write!(f, "bad lmd count: 0x{:08X}", u),
            Self::BadLmdPayload => write!(f, "bad lmd payload"),
            Self::BadLmdState => write!(f, "bad lmd state"),
            Self::BadPayloadCount => write!(f, "bad payload count"),
            Self::BadRawByteCount => write!(f, "bad raw byte count"),
            Self::BadReaderState => write!(f, "bad reader state"),
            Self::BadWeightPayload => write!(f, "bad weight payload"),
            Self::BadWeightPayloadCount => write!(f, "bad weight payload count"),
            Self::WeightPayloadOverflow => write!(f, "weight payload overflow"),
            Self::WeightPayloadUnderflow => write!(f, "weight payload underflow"),
        }
    }
}
