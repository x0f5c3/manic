use crate::Connector;
use crate::Error;
use hyper::header::RANGE;
use hyper::Client;
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
    pub fn new(low: u64, hi: u64, chunk_size: u32) -> Result<Self, Error> {
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

#[instrument(skip(client))]
pub async fn download(
    val: String,
    url: &str,
    client: &Client<impl Connector>,
) -> Result<Vec<u8>, Error> {
    let req = hyper::Request::get(url)
        .header(RANGE, val)
        .body(hyper::Body::empty())?;
    let resp = client.request(req.into()).await?.into_body();
    let bytes = hyper::body::to_bytes(resp).await?;
    Ok(bytes.as_ref().to_vec())
}
