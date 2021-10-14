use crate::downloader::join_all;
use crate::ManicError;
use crate::Result;
use rayon::prelude::*;
use std::io::Cursor;
use std::io::SeekFrom;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

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
    buf: Arc<Mutex<Cursor<Vec<u8>>>>,
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
    pub(crate) async fn as_vec(&self) -> Vec<u8> {
        self.buf.lock().await.clone().into_inner()
    }
    pub(crate) async fn from_vec(v: Vec<Chunk>) -> Result<Self> {
        let cur = Cursor::new(Vec::new());
        let wrapped = Arc::new(Mutex::new(cur));
        let fut_vec: Vec<_> = v
            .par_iter()
            .cloned()
            .map(|x: Chunk| tokio::spawn(write_cursor(wrapped.clone(), x)))
            .collect();
        Ok(Self {
            chunks: Arc::new(v),
            buf: wrapped,
        })
    }
}

async fn write_cursor(cur: Arc<Mutex<Cursor<Vec<u8>>>>, chunk: Chunk) -> Result<()> {
    let mut lock = cur.lock().await;
    lock.seek(SeekFrom::Start(chunk.low)).await?;
    lock.write_all(chunk.buf.lock().await.as_slice()).await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub buf: Arc<Mutex<Vec<u8>>>,
    pub low: u64,
    pub hi: u64,
    pub pos: u64,
    pub bytes: String,
}

impl Chunk {
    pub(crate) async fn save(self, mut output: File) -> Result<()> {
        output.seek(SeekFrom::Start(self.low)).await?;
        output
            .write_all(self.buf.lock().await.as_slice())
            .await
            .map_err(|x| x.into())
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
            self.current_pos += 1;
            Some(Chunk {
                buf: Arc::new(Mutex::new(vec![0; chunk_len as usize])),
                low: prev_low,
                hi: self.low - 1,
                pos: self.current_pos,
                bytes,
            })
        }
    }
}
