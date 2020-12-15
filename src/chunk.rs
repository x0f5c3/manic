use reqwest::Client;
use thiserror::Error;
use reqwest::header::RANGE;
#[cfg(feature = "progress")]
use indicatif::ProgressBar;
use tracing::instrument;


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






    #[cfg(not(feature = "progress"))]
    #[instrument(skip(client))]
    pub(crate) async fn download(val: String, url: &str, client: &Client) -> Result<Vec<u8>, Error> {
        let resp = client.get(url).header(RANGE, val).send().await?.bytes().await?;
        Ok(resp.as_ref().to_vec())
    }

    #[cfg(feature = "progress")]
    #[instrument(skip(client, pb))]
    pub(crate) async fn download(val: String, url: &str, client: &Client, pb: Option<ProgressBar>) -> Result<Vec<u8>, Error> {
        let mut res = Vec::new();
        let mut resp = client.get(url).header(RANGE, val).send().await?;
        if let Some(pb1) = pb {
        while let Some(chunk) = resp.chunk().await? {
            pb1.inc(chunk.len() as u64);
            res.append(&mut chunk.to_vec());
        }
        } else {
            while let Some(chunk) = resp.chunk().await? {
                res.append(&mut chunk.to_vec());
            }
        }
        Ok(res)
        
    }


    






#[derive(Debug, Error)]
pub enum Error {
    #[error("Network error: {0}")]
    NetError(#[from] reqwest::Error),
    #[error("Invalid chunk size")]
    BadChunkSize,
    #[error("Error sending chunk")]
    SendError,
}






