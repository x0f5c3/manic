use std::path::Path;

use futures::{Future, StreamExt};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, instrument};

use crate::chunk::Chunks;
use crate::Client;
use crate::Error;
use crate::Hash;
use crate::Result;
use hyper::header::RANGE;
use hyper::Uri;

/// Main type of the crate, use to download the file and optionally verify it
#[derive(Debug)]
pub struct Downloader {
    /// Downloader hyper Client
    pub client: Client,
    hash: Option<Hash>,
    filename: String,
    chunks: Chunks,
    workers: u8,
    url: Uri,
    length: u64,
    #[cfg(feature = "progress")]
    bar: Option<indicatif::ProgressBar>,
}
impl Downloader {
    /// Create a downloader for a github asset
    #[cfg(feature = "github")]
    pub async fn new_from_asset(workers: u8, asset: crate::github::Asset) -> Result<Self> {
        let url = asset.browser_download_url;
        let size = asset.size as u64;
        let client = Client::new();
        Self::assemble(client, &url, workers, size).await
    }
    async fn assemble(client: Client, url: &str, workers: u8, size: u64) -> Result<Self> {
        let uri = url.parse::<Uri>()?;
        let filename = Self::get_filename(&uri)?;
        let chunks = Chunks::new(0, size - 1, size / workers as u64)?;
        #[cfg(not(feature = "progress"))]
        return Ok(Self {
            client,
            hash: None,
            filename,
            chunks,
            workers,
            url: uri,
            length: size,
        });
        #[cfg(feature = "progress")]
        return Ok(Self {
            client,
            hash: None,
            filename,
            chunks,
            workers,
            url: uri,
            length: size,
            bar: None,
        });
    }
    /// Create a new downloader
    ///
    /// # Arguments
    /// * `url` - URL of the file
    /// * `workers` - amount of concurrent tasks
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use manic::Downloader;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), manic::Error> {
    ///     let client = Downloader::new("https://crates.io", 5).await?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(url: &str, workers: u8) -> Result<Self> {
        let client = Client::new();
        let len = client.content_length(url).await?;
        debug!("Len: {}", len);
        Self::assemble(client, url, workers, len).await
    }
    /// Get filename from the url, returns an error if the url contains no filename
    #[instrument(skip(url),fields(URL=%url))]
    fn get_filename(url: &Uri) -> Result<String> {
        url.path()
            .split('/')
            .last()
            .and_then(|name| {
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            })
            .ok_or_else(|| Error::NoFilename("No filename".to_string()))
    }

    #[cfg(feature = "progress")]
    /// Set up the [`ProgressBar`][indicatif::ProgressBar]
    pub fn progress_bar(&mut self) {
        self.bar = Some(indicatif::ProgressBar::new(self.length));
    }

    #[cfg(feature = "progress")]
    /// Apply a [`ProgressStyle`][indicatif::ProgressStyle] to the [`ProgressBar`][indicatif::ProgressBar]
    pub fn bar_style(&self, style: indicatif::ProgressStyle) {
        if let Some(pb) = &self.bar {
            pb.set_style(style)
        }
    }
    /// Used to download and verify against a SHA256 sum,
    ///
    /// returns an error if the connection fails or if the sum doesn't match the one provided
    ///
    /// ```no_run
    /// # #[cfg(feature = "rustls-tls")]
    /// # use manic::Rustls;
    /// # use manic::downloader::Downloader;
    /// # use manic::Error;
    /// use manic::Hash;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    /// let mut dl = Downloader::new("https://crates.io", 5).await?;
    /// dl.verify(hash);
    /// let data = dl.download_and_verify().await?;
    /// # Ok(())
    /// # }
    /// ```
    /// "##
    /// )]
    ///
    #[instrument(skip(self))]
    pub async fn download_and_verify(&self) -> Result<Vec<u8>> {
        let data = self.download().await?;
        debug!("Downloaded");
        if let Some(sha) = &self.hash {
            sha.verify(&data)?;
            debug!("Compared");
        }
        Ok(data)
    }
    #[instrument(skip(self))]
    async fn download_chunk(&self, val: String) -> Result<Vec<u8>> {
        let mut res = Vec::new();
        let req = hyper::Request::get(&self.url)
            .header(RANGE, val)
            .body(hyper::Body::empty())?;
        let mut resp = self.request(req.into()).await?.0.into_body();
        while let Some(Ok(chunk)) = resp.next().await {
            #[cfg(feature = "progress")]
            if let Some(bar) = &self.bar {
                bar.inc(chunk.len() as u64);
            }
            res.append(&mut chunk.to_vec());
        }
        Ok(res)
    }

    /// Used to download, save to a file and verify against a SHA256 sum,
    /// returns an error if the connection fails or if the sum doesn't match the one provided
    ///
    /// # Arguments
    /// * `path` - where to download the file
    /// * `verify` - whether to verify the file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use manic::downloader::Downloader;
    /// # use manic::Error;
    /// # use manic::Hash;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    /// # let mut dl = Downloader::new("https://crates.io", 5).await?;
    /// dl.verify(hash);
    /// dl.download_and_save("~/Downloads/", true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_and_save(&self, path: &str, verify: bool) -> Result<()> {
        let mut result = {
            let original_path = Path::new(path);
            let file_path = if original_path.is_dir() {
                original_path.join(&self.filename)
            } else {
                original_path.to_path_buf()
            };
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
    /// Download the file
    ///
    /// # Example
    /// ```no_run
    /// # use manic::downloader::Downloader;
    /// # use manic::Error;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// # let dl = Downloader::new("https://crates.io", 5).await?;
    /// let result = dl.download().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub async fn download(&self) -> Result<Vec<u8>> {
        let mb = self.length / 1000000;
        debug!("File size: {}MB", mb);
        let hndl_vec = self
            .chunks
            .into_iter()
            .map(move |x| self.download_chunk(x))
            .collect::<Vec<_>>();
        debug!("Collected");
        let result: Vec<u8> = {
            let mut result = Vec::new();
            for i in hndl_vec {
                let mut curr_part = i.await?;
                result.append(&mut curr_part);
            }
            result
        };
        Ok(result)
    }
    /// Set the hash to verify against
    pub fn verify(&mut self, hash: Hash) -> &mut Self {
        self.hash = Some(hash);
        self
    }
}
