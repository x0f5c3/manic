use crate::chunk;
use reqwest::header::CONTENT_LENGTH;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::prelude::*;
use tokio::fs::File;
use tracing::{debug, instrument};
use crate::Error;

/// Get the content-length header using a head request
///
/// # Arguments
///
/// * `url` - &str with the url
/// * `client` - optional reference to a reqwest client in case custom settings are needed
///
/// # Examples
///
/// ```no_run
/// use manic::downloader;
/// use manic::Error;
/// use reqwest::Client;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
///     let client = Client::new();
///     let length = downloader::get_length("https://docs.rs", Some(&client)).await.unwrap();
///     assert_eq!(25853, length);
/// #   Ok(())
/// # }
/// ```
#[instrument(skip(client,url),fields(URL=%url))]
pub async fn get_length(url: &str, client: Option<&Client>) -> Result<u64, Error> {
    let head_req = {
        if let Some(cl) = client {
            cl.head(url).send().await?
        } else {
            let cl = Client::new();
            cl.head(url).send().await?
        }
    };
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
/// use manic::downloader;
/// use manic::Error;
/// # fn main() -> Result<(), Error> {
///     let name = downloader::get_filename("http://test.rs/test.zip")?;
///     assert_eq!("test.zip", name);
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
        .ok_or_else(|| Error::NoFilename(url.to_string()))
}
/// Compare SHA256 of the data to the given sum,
/// will return an error if the sum is not equal to the data's
/// # Arguments
/// * `data` - u8 slice of data to compare
/// * `hash` - SHA256 sum to compare to
///
/// # Examples
///
/// ```
/// use manic::downloader::compare_sha;
/// use manic::Error;
/// # fn main() -> Result<(), Error> {
///     let data: &[u8] = &[1,2,3];
///     let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81";
///     compare_sha(data,hash).unwrap();
/// # Ok(())
/// # }
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
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
/// * `client` - reference to a reqwest client
///
/// # Examples
///
/// ```no_run
/// use reqwest::Client;
/// use manic::downloader;
/// use manic::Error;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let client = Client::new();
/// let result = downloader::download(&client, "https://crates.io", 5).await?;
/// # Ok(())
/// # }
/// ```
#[instrument(skip(client, url, workers), fields(URL=%url, tasks=%workers))]
pub async fn download(client: &Client, url: &str, workers: u8) -> Result<Vec<u8>, Error> {
    let length = get_length(url, Some(&client)).await?;
    let mb = length / 1000000;
    debug!("File size: {}MB", mb);
    let chunk_iter = chunk::ChunkIter::new(0, length - 1, (length / workers as u64) as u32)?;
    let hndl_vec = chunk_iter
        .into_iter()
        .map(move |x| {
            chunk::download(x, url, client)
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

/// Used to download and verify against a SHA256 sum,
/// returns an error if the connection fails or if the sum doesn't match the one provided
///
/// # Arguments
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
/// * `hash` - SHA256 sum to compare to
///
/// # Examples
///
/// ```no_run
/// use reqwest::Client;
/// use manic::downloader;
/// use manic::Error;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// # let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81";
/// let client = Client::new();
/// let data = downloader::download_and_verify(&client,"https://crates.io", 5, hash).await?;
/// # Ok(())
/// # }
/// ```
///
#[instrument(skip(client))]
pub async fn download_and_verify(
    client: &Client,
    url: &str,
    workers: u8,
    hash: &str,
) -> Result<Vec<u8>, Error> {
    let data = download(client, url, workers).await?;
    debug!("Downloaded");
    compare_sha(data.as_slice(), hash)?;
    debug!("Compared");
    Ok(data)
}


/// Used to download, save to a file and verify against a SHA256 sum,
/// returns an error if the connection fails or if the sum doesn't match the one provided
///
/// # Arguments
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
/// * `hash` - SHA256 sum to compare to
///
/// # Examples
///
/// ```no_run
/// use manic::downloader;
/// use manic::Error;
/// use reqwest::Client;
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     let client = Client::new();
///     let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81";
///     let data = downloader::download_verify_and_save(&client, "https://crates.io", 5, hash, "~/Downloads").await?;
///     Ok(())
///  }
/// ```
///
#[instrument(skip(client))]
pub async fn download_verify_and_save(
    client: &Client,
    url: &str,
    workers: u8,
    hash: &str,
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
    compare_sha(data.as_slice(), hash)?;
    Ok(())
}


