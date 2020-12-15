use reqwest::Client;
use reqwest::header::RANGE;
use indicatif::ProgressBar;
use tracing::instrument;
use crate::Error;


#[derive(Debug)]
pub(crate) struct ChunkIter {
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







    #[instrument(skip(client, pb))]
    pub(crate) async fn download(val: String, url: &str, client: &Client, pb: ProgressBar) -> Result<Vec<u8>, Error> {
        let mut res = Vec::new();
        let mut resp = client.get(url).header(RANGE, val).send().await?;
        while let Some(chunk) = resp.chunk().await? {
            pb.inc(chunk.len() as u64);
            res.append(&mut chunk.to_vec());
        }
        Ok(res)
        
    }


    












