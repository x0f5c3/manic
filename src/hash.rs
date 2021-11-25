use crate::{ManicError, Result};
use derive_more::Display;
use md5::Md5;
use sha2::Digest;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use tracing::debug;

/// Available checksum types
#[derive(Debug, Clone, Display)]
#[display(fmt = "{}")]
pub enum Hash {
    /// MD5 sum
    #[display(fmt = "_0")]
    MD5((Md5, String)),
    /// Sha224 sum
    #[display(fmt = "_0")]
    SHA224((Sha224, String)),
    /// Sha256 sum
    #[display(fmt = "_0")]
    SHA256((Sha256, String)),
    /// Sha384 sum
    #[display(fmt = "_0")]
    SHA384((Sha384, String)),
    /// Sha512 sum
    #[display(fmt = "_0")]
    SHA512((Sha512, String)),
}
impl Hash {
    /// New SHA224 hash value
    pub fn new_sha224(to_verify: String) -> Self {
        Self::SHA224((Sha224::new(), to_verify))
    }
    /// New SHA256 hash value
    pub fn new_sha256(to_verify: String) -> Self {
        Self::SHA256((Sha256::new(), to_verify))
    }
    /// New SHA384 hash value
    pub fn new_sha384(to_verify: String) -> Self {
        Self::SHA384((Sha384::new(), to_verify))
    }
    /// New SHA512 hash value
    pub fn new_sha512(to_verify: String) -> Self {
        Self::SHA512((Sha512::new(), to_verify))
    }
    /// Finalize the hasher and return the hex string of the final value
    pub fn finalize(self) -> String {
        match self {
            Self::SHA256((h, _)) => format!("{:x}", h.finalize()),
            Self::SHA224((h, _)) => format!("{:x}", h.finalize()),
            Self::SHA512((h, _)) => format!("{:x}", h.finalize()),
            Self::SHA384((h, _)) => format!("{:x}", h.finalize()),
            Self::MD5((h, _)) => format!("{:x}", h.finalize()),
        }
    }
    /// Check if computed sum matches the reference
    pub fn verify(self) -> Result<()> {
        let hash_string = format!("{}", self);
        debug!("Comparing sum {}", hash_string);
        let to_verify = self.finalize();
        debug!("SHA256 sum: {}", to_verify);
        if to_verify == hash_string {
            debug!("SHAsum match!");
            Ok(())
        } else {
            Err(ManicError::SHA256MisMatch(to_verify))
        }
    }
    /// Update the hasher with data
    pub fn update(&mut self, data: &[u8]) {
        match self {
            Self::SHA256((h, _)) => h.update(data),
            Self::SHA224((h, _)) => h.update(data),
            Self::SHA384((h, _)) => h.update(data),
            Self::SHA512((h, _)) => h.update(data),
            Self::MD5((h, _)) => h.update(data),
        }
    }
}
