use std::fmt;

use crate::{Result, Error};
use tracing::debug;
use sha2::{Sha256, Sha224, Sha512, Sha384, Digest};

/// Available checksum types
#[derive(Debug)]
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
    /// Compare SHA256 of the data to the given sum,
    /// will return an error if the sum is not equal to the data's
    /// # Arguments
    /// * `data` - u8 slice of data to compare
    ///
    /// # Example
    ///
    /// ```
    /// use manic::Error;
    /// use manic::Hash;
    /// # fn main() -> Result<(), Error> {
    ///     let data: &[u8] = &[1,2,3];
    ///     let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    ///     hash.verify(data)?;
    /// # Ok(())
    /// # }
    /// ```
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