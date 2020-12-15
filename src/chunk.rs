use reqwest::Client;
use reqwest::header::RANGE;
use tracing::instrument;
use crate::Error;


#[derive(Debug)]
pub struct ChunkIter {
    low: u64,
    hi: u64,
    chunk_size: u32,
}

impl ChunkIter {
    pub fn new(low: u64, hi: u64, chunk_size: u32) -> Result<Self, Error> {
        if chunk_size == 0 {
            return Err(Error::BadChunkSize)
        }
        Ok(ChunkIter {
            low,
            hi,
            chunk_size,
        })
    }
}

impl Iterator for ChunkIter {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        if self.low > self.hi {
            None
        } else {
            let prev_low = self.low;
            self.low += std::cmp::min(self.chunk_size as u64, self.hi - self.low + 1);
            Some(format!("bytes={}-{}", prev_low, self.low - 1))
        }
    }
}






    #[instrument(skip(client))]
    pub async fn download(val: String, url: &str, client: &Client) -> Result<Vec<u8>, Error> {
        let resp = client.get(url).header(RANGE, val).send().await?.bytes().await?;
        Ok(resp.as_ref().to_vec())
    }


    












