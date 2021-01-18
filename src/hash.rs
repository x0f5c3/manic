use std::fmt;
use tracing::debug;
use crate::{Error, Result};
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha2::Digest;

/// Available checksum types
#[derive(Debug, Clone)]
pub enum Hash {
    /// Sha224 sum
    SHA224(String),
    /// Sha256 sum
    SHA256(String),
    /// Sha384 sum
    SHA384(String),
    /// Sha512 sum
    SHA512(String),
}
impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SHA224(val) | Self::SHA256(val) | Self::SHA384(val) | Self::SHA512(val) => {
                write!(f, "{}", val)
            }
        }
    }
}

impl Hash {
    pub fn verify(&self, data: &[u8]) -> Result<()> {
        let hash_string = format!("{}", self);
        debug!("Comparing sum {}", hash_string);
        let to_verify = match self {
            Self::SHA256(_) => format!("{:x}", Sha256::digest(data)),
            Self::SHA224(_) => format!("{:x}", Sha224::digest(data)),
            Self::SHA512(_) => format!("{:x}", Sha512::digest(data)),
            Self::SHA384(_) => format!("{:x}", Sha384::digest(data)),
        };
        debug!("SHA256 sum: {}", to_verify);
        if to_verify == hash_string {
            debug!("SHAsum match!");
            Ok(())
        } else {
            Err(Error::SHA256MisMatch(to_verify))
        }
    }
}
