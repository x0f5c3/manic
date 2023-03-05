use crate::error::CrocError;
use crate::error::Result;
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct Metadata {
    pub filesize: u64,
    pub filename: String,
    pub c_time: SystemTime,
    pub m_time: SystemTime,
}

/// Contains the metadata for all files that will be sent
/// during a particular transfer
#[derive(PartialEq, Debug, Clone)]
pub struct TransferInfo {
    /// The metadata to send to the peer. These
    /// filenames are striped of their path information
    pub all: Vec<Metadata>,

    /// Internal state for a sender to locate files
    pub localpaths: Vec<PathBuf>,
}

impl Decode for TransferInfo {
    fn decode<D: Decoder>(decoder: &mut D) -> std::result::Result<Self, DecodeError> {
        Ok(Self {
            all: Decode::decode(decoder)?,
            localpaths: Vec::new(),
        })
    }
}

impl Encode for TransferInfo {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> std::result::Result<(), EncodeError> {
        Encode::encode(&self.all, encoder)
    }
}

impl TransferInfo {
    /// Owned TransferInfo
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use std::error::Error;
    /// use portal_lib::TransferInfo;
    ///
    /// fn create_info(files: Vec<PathBuf>) -> Result<TransferInfo, Box<dyn Error>> {
    ///     let mut info = TransferInfo::empty();
    ///
    ///     for file in files {
    ///         info.add_file(file.as_path())?;
    ///     }
    ///
    ///     Ok(info)
    /// }
    /// ```
    pub fn new() -> TransferInfo {
        TransferInfo {
            all: Vec::new(),
            localpaths: Vec::new(),
        }
    }

    /// Add a file to this transfer
    pub fn add_file(&mut self, path: &Path) -> Result<&mut TransferInfo> {
        self.localpaths.push(path.to_path_buf());
        let meta = path.metadata()?;
        let filesize = meta.len();
        self.all.push(Metadata {
            filesize,
            filename: path
                .file_name()
                .ok_or(CrocError::BadFileName)?
                .to_str()
                .ok_or(CrocError::BadFileName)?
                .to_string(),
            c_time: meta.created()?,
            m_time: meta.modified()?,
        });
        Ok(self)
    }
}
