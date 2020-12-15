use crate::chunk;
#[cfg(feature = "progress")]
use indicatif::ProgressBar;
use reqwest::header::CONTENT_LENGTH;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::num::ParseIntError;
use std::path::Path;
use thiserror::Error;
use tokio::prelude::*;
use tokio::{fs::File, io};
use tracing::{debug, instrument};

/// Get the content-length header using a head request
///
/// # Arguments
///
/// * `url` - &str with the url
/// * `client` - reference to a reqwest client
///
/// # Examples
///
/// ```no_run
/// use reqwest::Client;
/// use par_download::download;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), download::Error> {
///     let client = Client::new();
///     let length = download::get_length("https://docs.rs", &client).await.unwrap();
///     assert_eq!(25853, length);
/// #   Ok(())
/// # }
/// ```
#[instrument(skip(client,url),fields(URL=%url))]
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
/// use par_download::download;
/// # fn main() -> Result<(), download::Error> {
/// let name = download::get_filename("http://test.rs/test.zip")?;
/// assert_eq!("test.zip", name);
/// # Ok(())
/// # }
/// ```
#[instrument]
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
/// Compare SHA256 of the data to the given sum
/// Will return an error if the sum is not equal to the data's
/// # Arguments
/// * `data` - u8 slice of data to compare
/// * `hash` - SHA256 sum to compare to
///
/// # Examples
///
/// ```
/// use par_download::download::compare_sha;
/// let data: &[u8] = &[1,2,3];
/// let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string();
/// compare_sha(data,hash).unwrap();
/// ```
#[instrument(skip(data, hash), fields(SHA=%hash))]
pub fn compare_sha(data: &[u8], hash: &str) -> Result<(), Error> {
    debug!("Comparing sum {}", hash);
    let finally = Sha256::digest(data);
    let hexed = format!("{:x}", finally);
    debug!("SHA256 sum: {}", hexed);
    if hexed == hash {
        debug!("SHA256 MATCH!");
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
/// ```no_run
/// use reqwest::Client;
/// use par_download::download;
/// # #[tokio::main]
/// # async fn main() -> Result<(), download::Error> {
/// let client = Client::new();
/// let result = par_download::download::download(&client, "https://crates.io", 5).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(not(feature = "progress"))]
#[instrument(skip(client, url, workers), fields(URL=%url, tasks=%workers))]
pub async fn download(client: &Client, url: &str, workers: u8) -> Result<Vec<u8>, Error> {
    let length = get_length(url, client).await?;
    let mb = length / 1000000;
    debug!("File size: {}MB", mb);
    let chunk_iter = ChunkIter::new(0, length - 1, (length / workers as u64) as u32)?;
    let hndl_vec = chunk_iter
        .into_iter()
        .map(move |x| {
            let hndl = chunk::download(x, url, client);
            hndl
        })
        .collect::<Vec<_>>();
    let result: Vec<u8> = {
        let mut result = Vec::new();
        for i in hndl_vec {
            let mut curr_part = i
                .await
                .map_err(|_| Error::SHA256MisMatch("Failed".to_string()))?;
            result.append(&mut curr_part);
        }
        result
    };

    Ok(result)
}
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
/// * `pb` - optional progress bar
///
/// # Examples
///
/// ```no_run
/// use reqwest::Client;
/// use indicatif::ProgressBar;
/// use par_download::download;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), download::Error> {
/// let pb = ProgressBar::new(100);
/// let client = Client::new();
/// let url = "https://crates.io";
/// let result = download::download(client, url, 5, Some(pb)).await?;
/// # Ok(())
/// # }
/// ```
///
#[cfg(feature = "progress")]
#[instrument(skip(client, pb))]
pub async fn download(
    client: &Client,
    url: &str,
    workers: u8,
    pb: Option<ProgressBar>,
) -> Result<Vec<u8>, Error> {
    let length = get_length(url, client).await?;
    let mb = length / 1000000;
    let chunk_len = length / workers as u64;
    let mut fut_ordered = Vec::new();
    let chunk_iter = chunk::ChunkIter::new(0, length - 1, chunk_len as u32)?; 
    if let Some(pb1) = pb {
        pb1.set_length(length);
        pb1.println(format!("File size: {}MB", mb));
        pb1.println(format!("Chunk length: {}", chunk_len));
    for x in chunk_iter.into_iter() {

        let hndl = chunk::download(x, url, client, Some(pb1.clone()));

        fut_ordered.push(hndl);
    }
    } else {
        for x in chunk_iter.into_iter() {
            fut_ordered.push(chunk::download(x, url, client, None));
        }
    }
    let last: Vec<u8> = {
        let mut result = Vec::new();
        for i in fut_ordered {
            let mut curr_part = i
                .await
                .map_err(|_| Error::SHA256MisMatch("Failed".to_string()))?;
            result.append(&mut curr_part);
        }
        result
    };

    Ok(last)
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
/// ```no_run
/// use reqwest::Client;
/// use par_download::download;
/// # #[tokio::main]
/// # async fn main() -> Result<(), download::Error> {
/// let client = Client::new();
/// # let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string();
/// let data = download::download_and_verify(&client, "https://crates.io", 5, hash).await?;
/// # Ok(())
/// # }
/// ```
///
#[cfg(not(feature = "progress"))]
#[instrument(skip(client))]
pub async fn download_and_verify(
    client: &Client,
    url: &str,
    workers: u8,
    hash: String,
) -> Result<Vec<u8>, Error> {
    let data = download(client, url, workers).await?;
    debug!("Downloaded");
    // compare_sha(data.clone(), hash).await?;
    debug!("Compared");
    Ok(data)
}

#[cfg(feature = "progress")]
#[instrument(skip(client))]
pub async fn download_and_verify(
    client: &Client,
    url: &str,
    workers: u8,
    hash: &str,
    pb: Option<ProgressBar>,
) -> Result<Vec<u8>, Error> {
    let data = download(client, url, workers, pb).await?;
    debug!("Downloaded");
    compare_sha(data.as_slice(), hash)?;
    debug!("Compared");
    Ok(data)
}
#[cfg(not(feature = "progress"))]
#[instrument(skip(client))]
pub async fn download_verify_and_save(
    client: &Client,
    url: &str,
    workers: u8,
    hash: String,
    path: &str,
) -> Result<(), Error> {
    let mut result = {
        let name = get_filename(url)?;
        let file_path = Path::new(path).join(name);
        File::create(file_path).await?
    };
    let data = download(client, url, workers).await?;
    result.write_all(data.as_slice()).await?;
    result.sync_all().await?;
    result.flush().await?;
    // compare_sha(data, hash).await?;
    Ok(())
}

#[cfg(feature = "progress")]
#[instrument(skip(client))]
pub async fn download_verify_and_save(
    client: &Client,
    url: &str,
    workers: u8,
    hash: &str,
    path: &str,
    pb: Option<ProgressBar>,
) -> Result<(), Error> {
    let mut result = {
        let name = get_filename(url)?;
        let file_path = Path::new(path).join(name);
        File::create(file_path).await?
    };
    let data = download(client, url, workers, pb).await?;
    result.write_all(data.as_slice()).await?;
    result.sync_all().await?;
    result.flush().await?;
    compare_sha(data.as_slice(), hash)?;
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
    #[error(transparent)]
    ChunkErr(#[from] chunk::Error),
}
