use crate::chunk::{self, ChunkIter};
use crate::Error;
use crate::Hash;
use crate::Result;
use reqwest::header::CONTENT_LENGTH;
use reqwest::header::RANGE;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, instrument};

#[derive(Debug, Clone)]
pub struct Downloader {
    filename: String,
    client: Client,
    workers: u8,
    url: reqwest::Url,
    hash: Option<Hash>,
    length: u64,
    chunks: ChunkIter,
    #[cfg(feature = "progress")]
    pb: Option<indicatif::ProgressBar>,
}

impl Downloader {
    pub async fn new(url: &str, workers: u8) -> Result<Self> {
        let client = Client::new();
        let parsed = reqwest::Url::parse(url)?;
        let length = client
            .head(url)
            .send()
            .await?
            .content_length()
            .ok_or(Error::BadChunkSize)?;
        let chunks = ChunkIter::new(0, length - 1, (length / workers as u64) as u32)?;
        let filename = Self::get_filename(&parsed)?;
        #[cfg(not(feature = "progress"))]
        return Ok(Self {
            filename,
            client,
            workers,
            url: parsed,
            hash: None,
            length,
            chunks,
        });
        #[cfg(feature = "progress")]
        return Ok(Self {
            filename,
            client,
            workers,
            url: parsed,
            hash: None,
            length,
            chunks,
            pb: None,
        });
    }
    /// Get filename from the url, returns an error if the url contains no filename
    ///
    /// # Arguments
    ///
    /// * `url` - &str with the url
    ///
    /// # Example
    /// ```
    /// use manic::downloader;
    /// use manic::Error;
    /// # fn main() -> Result<(), Error> {
    ///     let name = downloader::get_filename("http://test.rs/test.zip")?;
    ///     assert_eq!("test.zip", name);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_filename(url: &reqwest::Url) -> Result<String> {
        url.path_segments()
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
    #[cfg(feature = "progress")]
    pub fn progress_bar(&mut self) {
        self.pb = Some(indicatif::ProgressBar::new(self.length));
    }
    #[cfg(feature = "progress")]
    pub fn bar_style(&self, style: indicatif::ProgressStyle) {
        if let Some(pb) = &self.pb {
            pb.set_style(style);
        }
    }
    pub fn to_verify(&mut self, hash: Hash) {
        self.hash = Some(hash);
    }
    #[instrument(skip(self))]
    async fn download_chunk(&self, val: String) -> Result<Vec<u8>> {
        let mut resp = self
            .client
            .get(&self.url.to_string())
            .header(RANGE, val)
            .send()
            .await?;
        #[cfg(not(feature = "progress"))]
        return Ok(resp.bytes().await?.as_ref().to_vec());
        #[cfg(feature = "progress")]
        {
            let mut res = Vec::new();
            while let Some(chunk) = resp.chunk().await? {
                if let Some(bar) = &self.pb {
                    bar.inc(chunk.len() as u64);
                }
                res.append(&mut chunk.to_vec());
            }
            return Ok(res);
        }
    }
    /// Download the file
    /// # Arguments
    /// * `client` - reference to a reqwest [`Client`][reqwest::Client]
    /// * `url` - &str with the url
    /// * `workers` - amount of concurrent downloads
    ///
    /// # Example
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
    #[instrument(skip(self), fields(URL=%self.url, tasks=%self.workers))]
    pub async fn download(&self) -> Result<Vec<u8>> {
        let mb = self.length / 1000000;
        debug!("File size: {}MB", mb);
        let hndl_vec = self
            .chunks
            .into_iter()
            .map(move |x| self.download_chunk(x))
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
    #[instrument(skip(self))]
    pub async fn download_and_verify(&self) -> Result<Vec<u8>> {
        let data = self.download().await?;
        debug!("Downloaded");
        if let Some(hash) = &self.hash {
            hash.verify(&data)?;
            debug!("Compared");
        }
        Ok(data)
    }
    pub async fn download_and_save(&self, path: &str, verify: bool) -> Result<()> {
        let mut result = {
            let file_path = Path::new(path).join(&self.filename);
            File::create(file_path).await?
        };
        let data = if verify {
            self.download_and_verify().await?
        } else {
            self.download().await?
        };
        result.write_all(data.as_slice()).await?;
        result.sync_all().await?;
        result.flush().await?;
        Ok(())
    }
}

/// Get the content-length header using a head request
///
/// # Arguments
///
/// * `url` - &str with the url
/// * `client` - optional reference to a reqwest [`Client`][reqwest::Client] in case custom settings are needed
///
/// # Example
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
pub async fn get_length(url: &str, client: Option<&Client>) -> Result<u64> {
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
/// # Example
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
pub fn get_filename(url: &str) -> Result<String> {
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
/// # Example
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
pub fn compare_sha(data: &[u8], hash: &str) -> Result<()> {
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
/// * `client` - reference to a reqwest [`Client`][reqwest::Client]
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
///
/// # Example
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
pub async fn download(client: &Client, url: &str, workers: u8) -> Result<Vec<u8>> {
    let length = get_length(url, Some(&client)).await?;
    let mb = length / 1000000;
    debug!("File size: {}MB", mb);
    let chunk_iter = ChunkIter::new(0, length - 1, (length / workers as u64) as u32)?;
    let hndl_vec = chunk_iter
        .into_iter()
        .map(move |x| chunk::download(x, url, client))
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
/// * `client` - reference to a reqwest [`Client`][reqwest::Client]
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
/// * `hash` - SHA256 sum to compare to
///
/// # Example
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
) -> Result<Vec<u8>> {
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
/// * `client` - reference to a reqwest [`Client`][reqwest::Client]
/// * `url` - &str with the url
/// * `workers` - amount of concurrent downloads
/// * `hash` - optional SHA256 sum to compare to
/// * `path` - where to download the file
/// * `verify` - set true to verify the file against the hash
///
/// # Example
///
/// ```no_run
/// use manic::downloader;
/// use manic::Error;
/// use reqwest::Client;
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     let client = Client::new();
///     let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81";
///     let data = downloader::download_and_save(&client, "https://crates.io", 5, Some(hash), "~/Downloads", true).await?;
///     Ok(())
///  }
/// ```
///
#[instrument(skip(client))]
pub async fn download_and_save(
    client: &Client,
    url: &str,
    workers: u8,
    hash: Option<&str>,
    path: &str,
    verify: bool,
) -> Result<()> {
    let mut result = {
        let name = get_filename(url)?;
        let file_path = Path::new(path).join(name);
        File::create(file_path).await?
    };
    let data = match hash {
        Some(sha) if verify => download_and_verify(client, url, workers, sha).await?,
        _ => download(client, url, workers).await?,
    };
    result.write_all(data.as_slice()).await?;
    result.sync_all().await?;
    result.flush().await?;
    Ok(())
}
