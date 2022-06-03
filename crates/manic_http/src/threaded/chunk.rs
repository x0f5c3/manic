use super::downloader::join_all;
use crate::header::RANGE;
use crate::threaded::Client;
use crate::Hash;
use crate::{ManicError, Result};
use bytes::Bytes;
#[cfg(feature = "progress")]
use indicatif::ProgressBar;
use rayon::prelude::*;
use rusty_pool::ThreadPool;
use std::fs::File;
use std::io::SeekFrom;
use std::io::{Seek, Write};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// Iterator over remote file chunks that returns a formatted [`RANGE`][reqwest::header::RANGE] header value
#[derive(Debug, Clone, Copy)]
pub struct Chunks {
    low: u64,
    hi: u64,
    chunk_size: u64,
    current_pos: u64,
}

#[derive(Debug, Clone)]
pub struct ChunkVec {
    chunks: Arc<Vec<Chunk>>,
}

impl ChunkVec {
    pub fn save_to_file<T: AsRef<Path>>(&self, path: T, pool: ThreadPool) -> Result<()> {
        let f = File::create(path)?;
        self.save(f, pool)
    }
    pub(crate) fn save(&self, output: File, pool: ThreadPool) -> Result<()> {
        let mut fut_vec = Vec::new();
        for i in self.chunks.iter() {
            let f = output.try_clone()?;
            let c = i.clone();
            fut_vec.push(pool.evaluate(|| c.save(f)))
        }
        join_all(fut_vec)?;
        output.sync_all()?;
        Ok(())
    }
    pub fn to_vec(&self) -> Vec<u8> {
        self.chunks
            .iter()
            .flat_map(|x| x.buf.to_vec())
            .collect::<Vec<u8>>()
    }
    pub(crate) fn verify(&self, mut hash: Hash) -> Result<()> {
        self.chunks.iter().for_each(|x| hash.update(x.buf.as_ref()));
        hash.verify()
    }
}

impl From<Vec<Chunk>> for ChunkVec {
    fn from(mut v: Vec<Chunk>) -> Self {
        v.par_sort_unstable_by(|a, b| a.pos.cmp(&b.pos));
        Self {
            chunks: Arc::new(v),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub buf: Bytes,
    pub low: u64,
    pub hi: u64,
    pub pos: u64,
    pub len: u64,
    pub bytes: String,
}

impl AsRef<Chunk> for Chunk {
    fn as_ref(&self) -> &Chunk {
        self
    }
}

impl Chunk {
    #[instrument(skip(self, output), fields(low = % self.low, hi = % self.hi, range = % self.bytes, pos = % self.pos))]
    pub(crate) fn save(self, mut output: File) -> Result<()> {
        output.seek(SeekFrom::Start(self.low))?;
        info!("Seeked");
        let n = output.write(self.buf.as_ref())?;
        info!("Written {} bytes", n);
        Ok(())
    }
    #[instrument(skip(self, client, pb), fields(range = % self.bytes))]
    pub(crate) fn download(
        mut self,
        client: Client,
        url: String,
        #[cfg(feature = "progress")] pb: Option<ProgressBar>,
    ) -> Result<Self> {
        let resp = client.get(url).header(RANGE, self.bytes.clone()).send()?;
        debug!(
            "Response headers: {:#?}\nResponse code: {}\nResponse reason: {:?}",
            resp.headers(),
            resp.status().as_u16(),
            resp.status().canonical_reason()
        );
        let res = resp.bytes()?;
        #[cfg(feature = "progress")]
        if let Some(bar) = pb {
            bar.inc(res.len() as u64);
        }
        self.buf = res;
        Ok(self)
    }
}

impl Chunks {
    /// Create the iterator
    /// # Arguments
    /// * `low` - the first byte of the file, typically 0
    /// * `hi` - the highest value in bytes, typically content-length - 1
    /// * `chunk_size` - the desired size of the chunks
    pub fn new(low: u64, hi: u64, chunk_size: u64) -> Result<Self> {
        if chunk_size == 0 {
            return Err(ManicError::BadChunkSize);
        }
        Ok(Chunks {
            low,
            hi,
            chunk_size,
            current_pos: 1,
        })
    }
    pub fn download(
        &self,
        client: Client,
        url: String,
        #[cfg(feature = "progress")] pb: Option<ProgressBar>,
        pool: ThreadPool,
    ) -> Result<ChunkVec> {
        let chnk_vec = self.collect::<Vec<Chunk>>();
        let fut_vec = chnk_vec
            .into_par_iter()
            .map(|x| {
                let client1 = client.clone();
                let url1 = url.clone();
                #[cfg(feature = "progress")]
                let pb1 = pb.clone();
                pool.evaluate(|| {
                    x.download(
                        client1,
                        url1,
                        #[cfg(feature = "progress")]
                        pb1,
                    )
                })
            })
            .collect::<Vec<_>>();
        let list = join_all(fut_vec)?;
        Ok(ChunkVec::from(list))
    }
}

impl Iterator for Chunks {
    type Item = Chunk;
    fn next(&mut self) -> Option<Self::Item> {
        if self.low > self.hi {
            None
        } else {
            let prev_low = self.low;
            self.low += std::cmp::min(self.chunk_size, self.hi - self.low + 1);
            let chunk_len = (self.low - 1) - prev_low;
            let bytes = format!("bytes={}-{}", prev_low, self.low - 1);
            let res = Chunk {
                buf: Bytes::new(),
                low: prev_low,
                hi: self.low - 1,
                len: chunk_len,
                pos: self.current_pos,
                bytes,
            };
            self.current_pos += 1;
            Some(res)
        }
    }
}
