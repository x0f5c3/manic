use crate::Error;
use reqwest::header::RANGE;
use reqwest::Client;
use tracing::instrument;

/// Iterator over remote file chunks that returns a formatted [`RANGE`][reqwest::header::RANGE] header value
#[derive(Debug, Clone, Copy)]
pub struct Chunks {
    low: u64,
    hi: u64,
    chunk_size: u64,
}

impl Chunks {
    /// Create the iterator
    /// # Arguments
    /// * `low` - the first byte of the file, typically 0
    /// * `hi` - the highest value in bytes, typically content-length - 1
    /// * `chunk_size` - the desired size of the chunks
    pub fn new(low: u64, hi: u64, chunk_size: u64) -> Result<Self, Error> {
        if chunk_size == 0 {
            return Err(Error::BadChunkSize);
        }
        Ok(Chunks {
            low,
            hi,
            chunk_size,
        })
    }
}

impl Iterator for Chunks {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        if self.low > self.hi {
            None
        } else {
            let prev_low = self.low;
            self.low += std::cmp::min(self.chunk_size, self.hi - self.low + 1);
            Some(format!("bytes={}-{}", prev_low, self.low - 1))
        }
    }
}

#[instrument(skip(client))]
pub async fn download(val: String, url: &str, client: &Client) -> Result<Vec<u8>, Error> {
    let resp = client
        .get(url)
        .header(RANGE, val)
        .send()
        .await?
        .bytes()
        .await?;
    Ok(resp.as_ref().to_vec())
}
