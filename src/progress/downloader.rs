use crate::downloader;
use crate::progress::chunk;
use crate::Error;
use indicatif::ProgressBar;
use reqwest::Client;
use std::path::Path;
use tokio::prelude::*;
use tokio::fs::File;
use tracing::{debug, instrument};

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
/// * `pb` - progress bar
///
/// # Examples
///
/// ```no_run
/// use reqwest::Client;
/// use indicatif::ProgressBar;
/// use manic::progress::downloader;
/// # use manic::Error;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let pb = ProgressBar::new(100);
/// let client = Client::new();
/// let url = "https://crates.io";
/// let workers: u8 = 5;
/// let result = downloader::download(&client, url, workers, pb).await?;
/// # Ok(())
/// # }
/// ```
///
#[instrument(skip(pb))]
pub async fn download(
    client: &Client,
    url: &str,
    workers: u8,
    pb: ProgressBar,
) -> Result<Vec<u8>, Error> {
    let length = downloader::get_length(url, Some(&client)).await?;
    let mb = length / 1000000;
    let chunk_len = length / workers as u64;
    let mut fut_vec = Vec::new();
    let chunk_iter = chunk::ChunkIter::new(0, length - 1, chunk_len as u32)?;
    pb.set_length(length);
    pb.println(format!("File size: {}MB", mb));
    pb.println(format!("Chunk length: {}", chunk_len));
    for x in chunk_iter.into_iter() {
        fut_vec.push(chunk::download(x, url, client, pb.clone()));
    }
    let last: Vec<u8> = {
        let mut result = Vec::new();
        for i in fut_vec {
            let mut curr_part = i.await?;
            result.append(&mut curr_part);
        }
        result
    };

    Ok(last)
}





/// Used to download and verify against a SHA256 sum,
/// returns an error if the connection fails or if the sum doesn't match the one provided
///
/// # Progress bar length
/// Progress bar length is set by the download function to the content-length of the file
///
/// # Arguments
/// * `client` - reference to a reqwest client
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
/// * `hash` - SHA256 sum to compare to
/// * `pb` - progress bar
///
/// # Examples
///
/// ```no_run
/// use manic::progress::downloader;
/// use reqwest::Client;
/// use indicatif::ProgressBar;
/// # use manic::Error;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// # let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81";
///     let client = Client::new();
///     let pb = ProgressBar::new(100);
///     let data = downloader::download_and_verify(&client, "https://crates.io", 5, hash, pb).await?;
/// # Ok(())
/// # }
/// ```
///
#[instrument(skip(pb))]
pub async fn download_and_verify(
    client: &Client,
    url: &str,
    workers: u8,
    hash: &str,
    pb: ProgressBar,
) -> Result<Vec<u8>, Error> {
    let data: Vec<u8> = download(client, url, workers, pb).await?;
    debug!("Downloaded");
    downloader::compare_sha(data.as_slice(), hash)?;
    debug!("Compared");
    Ok(data)
}

/// Used to download, save to a file and verify against a SHA256 sum,
/// returns an error if the connection fails or if the sum doesn't match the one provided
///
/// # Progress bar length
/// Progress bar length is set by the download function to the content-length of the file
///
/// # Arguments
/// * `client` - reference to a reqwest client
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
/// * `hash` - SHA256 sum to compare to
/// * `pb` - progress bar
///
/// # Examples
///
/// ```no_run
/// use reqwest::Client;
/// use manic::progress::downloader;
/// use indicatif::ProgressBar;
/// # use manic::Error;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let client = Client::new();
/// let pb = ProgressBar::new(100);
/// let path = "~/Downloads";
/// # let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81";
/// let data = downloader::download_verify_and_save(&client, "https://crates.io", 5, hash, path, pb).await?;
/// # Ok(())
/// # }
/// ```
///
#[instrument(skip(pb))]
pub async fn download_verify_and_save(
    client: &Client,
    url: &str,
    workers: u8,
    hash: &str,
    path: &str,
    pb: ProgressBar,
) -> Result<(), Error> {
    let mut result = {
        let name = downloader::get_filename(url)?;
        let file_path = Path::new(path).join(name);
        File::create(file_path).await?
    };
    let data = download(client, url, workers, pb).await?;
    result.write_all(data.as_slice()).await?;
    result.sync_all().await?;
    result.flush().await?;
    downloader::compare_sha(data.as_slice(), hash)?;
    Ok(())
}
