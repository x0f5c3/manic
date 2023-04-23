use crate::{CryptoError, Result};
use bincode::{Decode, Encode};
use ed25519_dalek::ed25519::signature;
use ed25519_dalek::ed25519::SignatureEncoding;
use std::fmt;
use std::str::FromStr;

/// Size of a single component of an Ed25519 signature.
const COMPONENT_SIZE: usize = 32;

/// Size of an `r` or `s` component of an Ed25519 signature when serialized
/// as bytes.
pub type ComponentBytes = [u8; COMPONENT_SIZE];

/// Ed25519 signature serialized as a byte array.
pub type SignatureBytes = [u8; Signature::BYTE_SIZE];

/// Ed25519 signature.
///
/// This type represents a container for the byte serialization of an Ed25519
/// signature, and does not necessarily represent well-formed field or curve
/// elements.
///
/// Signature verification libraries are expected to reject invalid field
/// elements at the time a signature is verified.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode)]
#[repr(C)]
pub struct Signature {
    r: ComponentBytes,
    s: ComponentBytes,
}

impl Signature {
    /// Size of an encoded Ed25519 signature in bytes.
    pub const BYTE_SIZE: usize = COMPONENT_SIZE * 2;

    /// Parse an Ed25519 signature from a byte slice.
    pub fn from_bytes(bytes: &SignatureBytes) -> Self {
        let mut r = ComponentBytes::default();
        let mut s = ComponentBytes::default();

        let components = bytes.split_at(COMPONENT_SIZE);
        r.copy_from_slice(components.0);
        s.copy_from_slice(components.1);

        Self { r, s }
    }

    /// Parse an Ed25519 signature from its `r` and `s` components.
    pub fn from_components(R: ComponentBytes, s: ComponentBytes) -> Self {
        Self { r: R, s }
    }

    /// Parse an Ed25519 signature from a byte slice.
    ///
    /// # Returns
    /// - `Ok` on success
    /// - `Err` if the input byte slice is not 64-bytes
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 64 {
            Err(CryptoError::InvalidLen(bytes.len()))
        }
        Self::try_from(bytes)
    }

    /// Bytes for the `r` component of a signature.
    pub fn r_bytes(&self) -> &ComponentBytes {
        &self.r
    }

    /// Bytes for the `s` component of a signature.
    pub fn s_bytes(&self) -> &ComponentBytes {
        &self.s
    }

    /// Return the inner byte array.
    pub fn to_bytes(&self) -> SignatureBytes {
        let mut ret = [0u8; Self::BYTE_SIZE];
        let (R, s) = ret.split_at_mut(COMPONENT_SIZE);
        R.copy_from_slice(&self.r);
        s.copy_from_slice(&self.s);
        ret
    }

    pub fn to_bytes_bincode(&self) -> Result<Vec<u8>> {
        bincode::encode_to_vec(&self, bincode::config::standard()).map_err(CryptoError::from)
    }

    pub fn from_bincode(buf: &[u8]) -> Result<Self> {
        let (ret, n): (Self, usize) = bincode::decode_from_slice(buf, bincode::config::standard())?;
        Ok(ret)
    }
    /// Convert this signature into a byte vector.
    #[cfg(feature = "alloc")]
    pub fn to_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl SignatureEncoding for Signature {
    type Repr = SignatureBytes;

    fn to_bytes(&self) -> SignatureBytes {
        self.to_bytes()
    }
}

impl From<Signature> for SignatureBytes {
    fn from(sig: Signature) -> SignatureBytes {
        sig.to_bytes()
    }
}

impl From<&Signature> for SignatureBytes {
    fn from(sig: &Signature) -> SignatureBytes {
        sig.to_bytes()
    }
}

impl From<SignatureBytes> for Signature {
    fn from(bytes: SignatureBytes) -> Self {
        Signature::from_bytes(&bytes)
    }
}

impl From<&SignatureBytes> for Signature {
    fn from(bytes: &SignatureBytes) -> Self {
        Signature::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = CryptoError;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        Self::from_slice(bytes)
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ed25519::Signature")
            .field("r", self.r_bytes())
            .field("s", self.s_bytes())
            .finish()
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self)
    }
}

impl fmt::LowerHex for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for component in [&self.R, &self.s] {
            for byte in component {
                write!(f, "{:02x}", byte)?;
            }
        }
        Ok(())
    }
}

impl fmt::UpperHex for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for component in [&self.R, &self.s] {
            for byte in component {
                write!(f, "{:02X}", byte)?;
            }
        }
        Ok(())
    }
}

// /// Decode a signature from hexadecimal.
// ///
// /// Upper and lower case hexadecimal are both accepted, however mixed case is
// /// rejected.
// // TODO(tarcieri): use `base16ct`?
// impl FromStr for Signature {
//     type Err = CryptoError;
//
//     fn from_str(hex: &str) -> Result<Self> {
//         if hex.as_bytes().len() != Signature::BYTE_SIZE * 2 {
//             return Err(CryptoError::InvalidLen(hex.as_bytes().len()));
//         }
//
//         let mut upper_case = None;
//
//         // Ensure all characters are valid and case is not mixed
//         for &byte in hex.as_bytes() {
//             match byte {
//                 b'0'..=b'9' => (),
//                 b'a'..=b'z' => match upper_case {
//                     Some(true) => return Err(CryptoError::new),
//                     Some(false) => (),
//                     None => upper_case = Some(false),
//                 },
//                 b'A'..=b'Z' => match upper_case {
//                     Some(true) => (),
//                     Some(false) => return Err(Error::new()),
//                     None => upper_case = Some(true),
//                 },
//                 _ => return Err(Error::new()),
//             }
//         }
//
//         let mut result = [0u8; Self::BYTE_SIZE];
//         for (digit, byte) in hex.as_bytes().chunks_exact(2).zip(result.iter_mut()) {
//             *byte = str::from_utf8(digit)
//                 .ok()
//                 .and_then(|s| u8::from_str_radix(s, 16).ok())
//                 .ok_or_else(Error::new)?;
//         }
//
//         Self::try_from(&result[..])
//     }
// }
