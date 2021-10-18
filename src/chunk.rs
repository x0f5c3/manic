use crate::cursor::MyCursor;
use crate::downloader::join_all;
use crate::header::RANGE;
use crate::{Client, Result};
use crate::{Hash, ManicError};
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::io::SeekFrom;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
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
    buf: MyCursor<Vec<u8>>,
}

impl ChunkVec {
    pub(crate) async fn save(&self, output: File) -> Result<()> {
        let mut fut_vec = Vec::new();
        for i in self.chunks.iter() {
            let f = output.try_clone().await?;
            let c = i.clone();
            fut_vec.push(tokio::spawn(c.save(f)))
        }
        join_all(fut_vec).await?;
        output.sync_all().await?;
        Ok(())
    }
    pub async fn as_vec(&self) -> Vec<u8> {
        self.buf.as_inner().await
    }
    pub(crate) async fn from_vec(v: Vec<Chunk>) -> Result<Self> {
        let wrapped = MyCursor::new(Vec::new());
        let fut_vec: Vec<_> = v
            .iter()
            .cloned()
            .map(|x: Chunk| tokio::spawn(write_cursor(wrapped.clone(), x)))
            .collect();
        join_all(fut_vec).await?;
        Ok(Self {
            chunks: Arc::new(v.par_iter().cloned().map(|x| x.clone()).collect()),
            buf: wrapped,
        })
    }
    pub(crate) async fn verify(&self, hash: &Hash) -> Result<()> {
        hash.verify(self.as_vec().await.as_slice())
    }
}
#[instrument(skip(cur, chunk), fields(low=%chunk.low, hi=%chunk.hi, range=%chunk.bytes, pos=%chunk.pos, len=%chunk.len))]
async fn write_cursor(cur: MyCursor<Vec<u8>>, chunk: Chunk) -> Result<()> {
    let mut lock = cur.lock().await;
    lock.seek(SeekFrom::Start(chunk.low)).await?;
    info!("Seeked");
    let n = lock.write(chunk.buf.lock().await.as_slice()).await?;
    info!("Written {} bytes", n);
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub buf: Arc<Mutex<Vec<u8>>>,
    pub low: u64,
    pub hi: u64,
    pub pos: u64,
    pub len: u64,
    pub bytes: String,
}

impl Chunk {
    #[instrument(skip(self, output), fields(low=%self.low, hi=%self.hi, range=%self.bytes, pos=%self.pos))]
    pub(crate) async fn save(self, mut output: File) -> Result<()> {
        output.seek(SeekFrom::Start(self.low)).await?;
        info!("Seeked");
        let n = output.write(self.buf.lock().await.as_slice()).await?;
        info!("Written {} bytes", n);
        Ok(())
    }
    #[instrument(skip(self, client, pb), fields(range = %self.bytes))]
    pub(crate) async fn download(
        self,
        client: Client,
        url: String,
        pb: Option<ProgressBar>,
    ) -> Result<Self> {
        let mut resp = client
            .get(url.to_string())
            .header(RANGE, self.bytes.clone())
            .send()
            .await?;
        {
            let mut res = self.buf.lock().await;
            while let Some(chunk) = resp.chunk().await? {
                #[cfg(feature = "progress")]
                if let Some(bar) = &pb {
                    bar.inc(chunk.len() as u64);
                }
                res.append(&mut chunk.to_vec());
            }
            Ok(self.to_owned())
        }
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
                tokio::spawn(x.download(
                    client.clone(),
                    url.clone(),
                    #[cfg(feature = "progress")]
                    pb.clone(),
                ))
            })
            .collect::<Vec<_>>();
        let list = join_all(fut_vec).await?;
        ChunkVec::from_vec(list).await
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
                buf: Arc::new(Mutex::new(Vec::new())),
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
