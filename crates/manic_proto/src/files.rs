use crc::CRC_16_IBM_SDLC;
use nix::sys::stat;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{ErrorKind, Read};
use std::os::unix::prelude::AsRawFd;
use std::path::Path;
use time::{Date, OffsetDateTime};

#[cfg(target_os = "windows")]
use windows::Storage::{FileProperties::BasicProperties, StorageFile};

#[cfg(target_os = "windows")]
pub fn get_attrs(path: String) -> Result<BasicProperties, Box<dyn std::error::Error>> {
    let f = StorageFile::GetFileFromPathAsync(path).await?;
}

pub struct FileOnDisk {
    meta:
}

pub const CHUNK_SIZE: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    Part {
        data: Vec<u8>,
        part_count: usize,
        part_no: usize,
        crc: u16,
        full_crc: u16,
    },
    Full {
        data: Vec<u8>,
        crc: u16,
    },
}

impl FileType {
    pub fn split(self, size: usize) -> Option<Vec<Self>> {
        if let Self::Full { data, crc: sum } = self {
            let split_data = data.chunks(size);
            let chunk_count = split_data.len();
            let parts = split_data
                .enumerate()
                .map(|(c, x)| {
                    let crc = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
                    let part_sum = crc.checksum(x);
                    Self::Part {
                        data: x.to_vec(),
                        part_count: chunk_count,
                        part_no: c,
                        crc: part_sum,
                        full_crc: sum,
                    }
                })
                .collect::<Vec<Self>>();
            Some(parts)
        } else {
            None
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum FullPack {
    Whole(File),
    Parts(Vec<File>),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Metadata {
    Part {
        name: String,
        full_path: String,
        size: i64,
        mode: Mode,
        access_time: TimeSpecs,
        part_count: usize,
        part_no: usize,
        crc: u16,
        full_crc: u16,
    },
    Full {
        name: String,
        full_path: String,
        size: i64,
        mode: Mode,
        access_time: TimeSpecs,
        crc: u16,
    },
}

impl Metadata {
    pub fn size(&self) -> i64 {
        *match &self {
            Self::Full { size, .. } => size,
            Self::Part { size, .. } => size,
        }
    }
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct File {
    meta: Metadata,
    data: Vec<u8>,
}

impl File {
    fn new(file_path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let file_name = Path::new(&file_path)
            .file_name()
            .and_then(|x| x.to_str())
            .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidData, "can't get filename"))?;

        let mut opened = fs::File::open(&file_path)?;
        let mut bytes = Vec::new();
        opened.read_to_end(&mut bytes)?;
        let stats: stat::FileStat = stat::fstat(opened.as_raw_fd())?;
        let crc = crc::Crc::<u16>::new(&CRC_16_IBM_SDLC);
        let crc = crc.checksum(&bytes);
        let size = stats.st_size;
        let mode = Mode(stats.st_mode);
        let a_time = TimeSpec {
            seconds: stats.st_atime,
            nanos: stats.st_atime_nsec,
        };
        let m_time = TimeSpec {
            seconds: stats.st_mtime,
            nanos: stats.st_mtime_nsec,
        };
        let times = TimeSpecs {
            atime: a_time,
            mtime: m_time,
        };
        let meta = Metadata::Full {
            name: file_name.to_string(),
            full_path: file_path,
            size,
            mode,
            access_time: times,
            crc,
        };
        Ok(Self { meta, data: bytes })
    }
    pub fn split(self, chunk_size: usize) -> Option<Vec<Self>> {
        if let Metadata::Full {
            name,
            full_path,
            size,
            mode,
            access_time,
            crc: sum,
        } = self.meta
        {
            let split_data = self.data.chunks(chunk_size);
            let chunk_count = split_data.len();
            let parts = split_data
                .enumerate()
                .map(|(c, x)| {
                    let crc = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
                    let part_sum = crc.checksum(x);
                    let meta = Metadata::Part {
                        name: name.clone(),
                        full_path: full_path.clone(),
                        size,
                        mode,
                        access_time,
                        part_count: chunk_count,
                        part_no: c,
                        crc: part_sum,
                        full_crc: sum,
                    };
                    Self {
                        meta,
                        data: x.to_vec(),
                    }
                })
                .collect::<Vec<Self>>();
            Some(parts)
        } else {
            None
        }
    }
}

impl FullPack {
    pub fn new(
        file_path: String,
        chunk_size: Option<usize>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let f = File::new(file_path)?;
        let chunk_size = chunk_size.unwrap_or(1024);
        if f.meta.size() > chunk_size as i64 {
            let res = f.split(size).unwrap();
            Ok(Self::Parts(res))
        } else {
            Ok(Self::Whole(f))
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Mode(stat::mode_t);

impl Mode {
    fn as_nix(&self) -> Option<stat::Mode> {
        stat::Mode::from_bits(self.0)
    }
}

pub struct FileAttrs {
    created: OffsetDateTime,
    accessed: OffsetDateTime,
    modified: OffsetDateTime,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct TimeSpecs {
    atime: TimeSpec,
    mtime: TimeSpec,
}
#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct TimeSpec {
    seconds: i64,
    nanos: i64,
}

impl From<nix::sys::time::TimeSpec> for TimeSpec {
    fn from(t: nix::sys::time::TimeSpec) -> Self {
        Self {
            seconds: t.tv_sec(),
            nanos: t.tv_nsec(),
        }
    }
}
