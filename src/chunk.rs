use crate::Error;
use crate::Result;
use hyper::client::connect::Connect;
use hyper::header::RANGE;
use hyper::{Client, Uri};
use tokio_stream::StreamExt;
use tracing::instrument;

/// Iterator over remote file chunks that returns a formatted [`RANGE`][hyper::header::RANGE] header value
#[derive(Debug, Copy, Clone)]
pub struct Chunks {
    low: u64,
    hi: u64,
    chunk_size: u32,
}

impl Chunks {
    /// Create the iterator
    /// # Arguments
    /// * `low` - the first byte of the file, typically 0
    /// * `hi` - the highest value in bytes, typically content-length - 1
    /// * `chunk_size` - the desired size of the chunks
    pub fn new(low: u64, hi: u64, chunk_size: u32) -> Result<Self> {
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
            self.low += std::cmp::min(self.chunk_size as u64, self.hi - self.low + 1);
            Some(format!("bytes={}-{}", prev_low, self.low - 1))
        }
    }
}
#[cfg(not(feature = "progress"))]
#[instrument(skip(client))]
pub async fn download<C>(val: String, url: &Uri, client: &Client<C>) -> Result<Vec<u8>>
where
    C: Connect + Send + Sync + Clone + 'static,
{
    let mut res = Vec::new();
    let req = hyper::Request::get(url)
        .header(RANGE, val)
        .body(hyper::Body::empty())?;
    let mut resp = client.request(req.into()).await?.into_body();
    while let Some(Ok(chunk)) = resp.next().await {
        res.append(&mut chunk.to_vec());
    }
    Ok(res)
}
#[cfg(feature = "progress")]
#[instrument(skip(client))]
pub async fn download<C>(
    val: String,
    url: &Uri,
    client: &Client<C>,
    pb: &Option<indicatif::ProgressBar>,
) -> Result<Vec<u8>>
where
    C: Connect + Send + Sync + Clone + 'static,
{
    let mut res = Vec::new();
    let req = hyper::Request::get(url)
        .header(RANGE, val)
        .body(hyper::Body::empty())?;
    let mut resp = client.request(req.into()).await?.into_body();
    while let Some(Ok(chunk)) = resp.next().await {
        if let Some(bar) = pb {
            bar.inc(chunk.len() as u64);
        }
        res.append(&mut chunk.to_vec());
    }
    Ok(res)
}
