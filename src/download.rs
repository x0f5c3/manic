use crate::chunk::{Chunk, Range};
use futures::stream::StreamExt;
use futures::stream::{self, FuturesOrdered};
use log::debug;
use std::path::Path;
use reqwest::header::CONTENT_LENGTH;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::num::ParseIntError;
use thiserror::Error;
use tokio::prelude::*;
use tokio::{fs::File, io};
#[cfg(feature = "progress")]
use indicatif::ProgressBar;


/// Get the content-length header using a head request
///
/// # Arguments
///
/// * `url` - &str with the url
/// * `client` - reference to a reqwest client
///
/// # Examples
///
/// ```
/// use reqwest::Client;
/// use par_download::download;
///
/// let client = Client::new();
/// let length = download::get_length("docs.rs", &client).await.unwrap();
/// assert_eq!(183, length);
/// ```
pub async fn get_length(url: &str, client: &Client) -> Result<u64, Error> {
    let head_req = client.head(url).send().await?;
    let headers = head_req.headers();
    debug!("Received head response: {:?}", headers);
    headers[CONTENT_LENGTH]
        .to_str()?
        .parse::<u64>()
        .map_err(Into::into)
}

/// Get filename from the url, returns an error if the url contains no filename
///
/// # Arguments
///
/// * `url` - &str with the url
///
/// # Examples
/// ```
/// let name = par_download::download::get_filename("http://test.rs/test.zip").unwrap();
/// assert_eq!("test.zip", name);
/// ```
pub fn get_filename(url: &str) -> Result<String, Error> {
    let parsed_url = reqwest::Url::parse(url)?;
    parsed_url
        .path_segments()
        .and_then(|segments| segments.last())
        .and_then(|name| {
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .ok_or(Error::NoFilename(url.to_string()))
}
/// Compare SHA256 of the data to the given hash
/// # Arguments
/// * `data` - u8 slice of data to compare
/// * `hash` - SHA256 sum to compare to
///
/// # Examples
///
/// ```
/// use par_download::download::compare_sha;
/// assert_eq!((), compare_sha(data, hash).unwrap());
/// ```
pub fn compare_sha(data: &[u8], hash: String) -> Result<(), Error> {
    let mut hash1 = Sha256::new();
    hash1.update(data);
    let finally = hash1.finalize();
    let hexed = format!("{:x}", finally);
    if hexed == hash {
        Ok(())
    } else {
        Err(Error::SHA256MisMatch(hexed))
    }
}


/// Download the file
/// # Arguments
/// * `client` - reference to a reqwest client
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
///
/// # Examples
///
/// ```
/// let result = par_download::download::download(client, url, workers).await?;
/// ```

pub async fn download(client: &Client, url: &str, workers: usize) -> Result<Vec<u8>, Error> {
    let length = get_length(url, client).await?;
    let mb = length / 1000000;
    let arr: Vec<u64> = (0..=mb).collect();
    Ok(stream::iter(arr.chunks(mb as usize / workers))
        .map(|x| async move {
            let low = x.first().unwrap();
            let hi = x.last().unwrap();
            let val = if low == hi {
                Range::Last(*hi)
            } else {
                Range::Normal((*low, *hi))
            };
            let chunk = Chunk::new(url, val);
            chunk.download(client)
        })
        .collect::<FuturesOrdered<_>>()
        .await
        .collect::<FuturesOrdered<_>>()
        .await
        .collect::<Vec<Result<Vec<u8>, crate::chunk::Error>>>()
        .await
        .into_iter()
        .filter_map(|x| x.ok())
        .collect::<Vec<Vec<u8>>>()
        .into_iter()
        .flatten()
        .collect())
}
#[cfg(feature = "progress")]
#[doc(cfg(feature = "progress"))]
/// Download the file with a progress bar using indicatif.
/// 
/// The function will set the length of the progress bar
/// to the content-length, the bar passed to the function can be initialized with any size
///
/// Only on feature "indicatif"
///
/// # Arguments
/// * `client` - reference to a reqwest client
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
///
/// # Examples
///
/// ```
/// use reqwest::Client;
/// use indicatif::ProgressBar;
/// use par_download::download;
/// 
/// let pb = ProgressBar::new(100);
/// let client = Client::new();
/// let url = "https://crates.io";
/// let result = download::download_with_progress(client, url, 5, pb).await?;
/// ```
/// 
pub async fn download_with_progress(client: &Client, url: &str, workers: usize, pb: ProgressBar) -> Result<Vec<u8>, Error> {
    let length = get_length(url, client).await?;
    pb.set_length(length);
    let mb = length / 1000000;
    let mut fut_ordered = FuturesOrdered::new();
    let arr: Vec<u64> = (0..=mb).collect();
    for x in arr.chunks(mb as usize / workers) {
            let low = *x.clone().first().unwrap();
            let pb1 = pb.clone();
            let hi = *x.clone().last().unwrap();
            let val = if low == hi {
                Range::Last(hi)
            } else {
                Range::Normal((low, hi))
            };
            let chunk = Chunk::new(url, val);
            fut_ordered.push(chunk.download_with_progress(client, pb1.clone()));
        }
    Ok(fut_ordered
        .collect::<Vec<Result<Vec<u8>, crate::chunk::Error>>>()
        .await
        .into_iter()
        .filter_map(|x| x.ok())
        .collect::<Vec<Vec<u8>>>()
        .into_iter()
        .flatten()
        .collect())
}

/// Used to download and verify against a SHA256 sum,
/// returns an error if the connection fails or if the sum doesn't match the one provided
///
/// # Arguments
/// * `client` - reference to a reqwest client
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
/// * `hash` - SHA256 sum to compare to
///
/// # Examples
///
/// ```
/// use reqwest::Client;
/// use par_download::download;
/// 
/// let client = Client::new();
/// let data = download::download_and_verify(&client, "https://crates.io", 5, hash).await?;
/// ```
///
pub async fn download_and_verify(client: &Client, url: &str, workers: usize, hash: String) -> Result<Vec<u8>, Error> {
    let data = download(client, url, workers).await?;
    compare_sha(data.as_slice(), hash)?;
    Ok(data)
}

async fn download_verify_and_save(client: &Client, url: &str, workers: usize, hash: String, path: &str) -> Result<(), Error> {
    let mut result = {
        let name = get_filename(url)?;
        let file_path = Path::new(path).join(name);
        File::create(file_path).await?
    };
    let data = download_and_verify(client, url, workers, hash).await?;
    result.write_all(data.as_slice()).await?;
    result.sync_all().await?;
    result.flush().await?;
    Ok(())
}

/// Error definition for possible errors in the download module
#[derive(Debug, Error)]
pub enum Error {
    /// Returned when the content length couldn't be parsed
    #[error("Failed to parse content-length")]
    LenParse(#[from] ParseIntError),
    /// Represents problems with Tokio based IO
    #[error("Tokio IO error: {0}")]
    TokioIOError(#[from] io::Error),
    /// Represents problems with network connectivity
    #[error("Reqwest error: {0}")]
    NetError(#[from] reqwest::Error),
    /// Returned when the header can't be parsed to a String
    #[error(transparent)]
    ToStr(#[from] reqwest::header::ToStrError),
    /// Returned when there's no filename in the url
    #[error("No filename in url")]
    NoFilename(String),
    /// Returned when the url couldn't be parsed
    #[error("Failed to parse URL")]
    UrlParseError(#[from] url::ParseError),
    /// Returned when the SHA256 sum didn't match
    #[error("Checksum doesn't match")]
    SHA256MisMatch(String),
}
