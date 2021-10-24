use crate::{ManicError, Result};
use sha2::Digest;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use std::fmt;
use tracing::debug;

/// Available checksum types
#[derive(Debug, Clone)]
pub enum Hash {
    /// Sha224 sum
    SHA224((Sha224, String)),
    /// Sha256 sum
    SHA256((Sha256, String)),
    /// Sha384 sum
    SHA384((Sha384, String)),
    /// Sha512 sum
    SHA512((Sha512, String)),
}
impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SHA224((_, val))
            | Self::SHA256((_, val))
            | Self::SHA384((_, val))
            | Self::SHA512((_, val)) => {
                write!(f, "{}", val)
            }
        }
    }
}

impl Hash {
    pub fn new_sha224(to_verify: String) -> Self {
        Self::SHA224((Sha224::new(), to_verify))
    }
    pub fn new_sha256(to_verify: String) -> Self {
        Self::SHA256((Sha256::new(), to_verify))
    }
    pub fn new_sha384(to_verify: String) -> Self {
        Self::SHA384((Sha384::new(), to_verify))
    }
    pub fn new_sha512(to_verify: String) -> Self {
        Self::SHA512((Sha512::new(), to_verify))
    }
    pub fn verify(self) -> Result<()> {
        let hash_string = format!("{}", self);
        debug!("Comparing sum {}", hash_string);
        let to_verify = match self {
            Self::SHA256((h, _)) => format!("{:x}", h.finalize()),
            Self::SHA224((h, _)) => format!("{:x}", h.finalize()),
            Self::SHA512((h, _)) => format!("{:x}", h.finalize()),
            Self::SHA384((h, _)) => format!("{:x}", h.finalize()),
        };
        debug!("SHA256 sum: {}", to_verify);
        if to_verify == hash_string {
            debug!("SHAsum match!");
            Ok(())
        } else {
            Err(ManicError::SHA256MisMatch(to_verify))
        }
    }
    pub fn update(&mut self, data: &[u8]) {
        match self {
            Self::SHA256((h, _)) => h.update(data),
            Hash::SHA224((h, _)) => h.update(data),
            Hash::SHA384((h, _)) => h.update(data),
            Hash::SHA512((h, _)) => h.update(data),
        }
    }
}
