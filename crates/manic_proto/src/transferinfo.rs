use crate::error::CodecError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

#[derive(Deserialize, Serialize, Encode, Decode, PartialEq, Debug, Clone)]
pub struct Metadata {
    pub filesize: u64,
    pub filename: String,
    #[bincode(with_serde)]
    pub c_time: OffsetDateTime,
    #[bincode(with_serde)]
    pub m_time: OffsetDateTime,
}

/// Contains the metadata for all files that will be sent
/// during a particular transfer
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TransferInfo {
    /// The metadata to send to the peer. These
    /// filenames are striped of their path information
    pub all: Vec<Metadata>,

    /// Internal state for a sender to locate files
    #[serde(skip)]
    pub localpaths: Vec<PathBuf>,
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
    pub fn add_file<'a>(&'a mut self, path: &Path) -> Result<&'a mut TransferInfo, Box<dyn Error>> {
        self.localpaths.push(path.to_path_buf());
        let meta = path.metadata()?;
        let filesize = meta.len();
        self.all.push(Metadata {
            filesize,
            filename: path
                .file_name()
                .ok_or(CodecError::BadFileName)?
                .to_str()
                .ok_or(CodecError::BadFileName)?
                .to_string(),
            c_time: meta.created()?.into(),
            m_time: meta.modified()?.into(),
        });
        Ok(self)
    }
}
