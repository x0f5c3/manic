use crate::cursor::MyCursor;
use crate::downloader::{join_all, join_all_futures};
use awc::http::header::RANGE;
use crate::Result;
use crate::downloader::Client;
use crate::{Hash, ManicError};
use futures::StreamExt;
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tracing::{info, instrument};

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
    pub async fn save_to_file<T: AsRef<Path>>(&self, path: T) -> Result<()> {
        let f = File::create(path).await?;
        self.save(f).await
    }
    pub(crate) async fn save(&self, output: File) -> Result<()> {
        let mut fut_vec = Vec::new();
        for i in self.chunks.iter() {
            let f = output.try_clone().await?;
            let c = i.clone();
            fut_vec.push(c.save(f))
        }
        join_all_futures(fut_vec).await?;
        output.sync_all().await?;
        Ok(())
    }
    // pub async fn as_vec(&self) -> Vec<u8> {
    //     self.chunks
    //         .iter()
    //         .par_bridge()
    //         .map(|x| x.buf.to_vec())
    //         .flatten().collect::<Vec<u8>>()
    // }
    pub(crate) async fn verify(&self, hash: &Hash) -> Result<()> {
        let input: Vec<u8> = self.chunks.iter().par_bridge().map(|x| x.buf.to_vec()).flatten().collect();
        hash.verify(input.as_slice())
    }
}

impl From<Vec<Chunk>> for ChunkVec {
    fn from(v: Vec<Chunk>) -> Self {
        Self {
            chunks: Arc::new(v),
        }
    }
}
#[instrument(skip(cur, chunk), fields(low=%chunk.as_ref().low, hi=%chunk.as_ref().hi, range=%chunk.as_ref().bytes, pos=%chunk.as_ref().pos, len=%chunk.as_ref().len))]
async fn write_cursor<C: AsRef<Chunk>>(cur: MyCursor<Vec<u8>>, chunk: C) -> Result<()> {
    let mut lock = cur.lock().await;
    lock.seek(SeekFrom::Start(chunk.as_ref().low)).await?;
    info!("Seeked");
    let n = lock.write(chunk.as_ref().buf.as_slice()).await?;
    info!("Written {} bytes", n);
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub buf: Vec<u8>,
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
    #[instrument(skip(self, output), fields(low=%self.low, hi=%self.hi, range=%self.bytes, pos=%self.pos))]
    pub(crate) async fn save(self, mut output: File) -> Result<()> {
        output.seek(SeekFrom::Start(self.low)).await?;
        info!("Seeked");
        let n = output.write(self.buf.as_slice()).await?;
        info!("Written {} bytes", n);
        Ok(())
    }
    #[instrument(skip(self, client, pb), fields(range = %self.bytes))]
    pub(crate) async fn download(
        mut self,
        client: Client,
        url: String,
        pb: Option<ProgressBar>,
    ) -> Result<Self> {
        let resp = client
            .get(url.to_string())
            .insert_header((RANGE, self.bytes.clone()))
            .send()
            .await?;
        let mut res: Vec<u8> = resp
            .filter_map(
                |x: std::result::Result<bytes::Bytes, awc::error::PayloadError>| async {
                    if let Ok(byt) = x {
                        #[cfg(feature = "progress")]
                        if let Some(bar) = &pb {
                            bar.inc(byt.len() as u64);
                        }
                        return Some(byt.to_vec());
                    }
                    None
                },
            )
            .collect::<Vec<Vec<u8>>>()
            .await
            .into_iter()
            .flatten()
            .collect();
        self.buf.append(&mut res);
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
    pub async fn download(
        &self,
        client: Client,
        url: String,
        pb: Option<ProgressBar>,
    ) -> Result<ChunkVec> {
        let fut_vec = self
            .map(|x| {
                x.download(
                    client.clone(),
                    url.clone(),
                    #[cfg(feature = "progress")]
                    pb.clone(),
                )
            })
            .collect::<Vec<_>>();
        let list = join_all_futures(fut_vec).await?;
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
                buf: Vec::new(),
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
