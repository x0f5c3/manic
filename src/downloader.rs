use crate::chunk::Chunks;
use crate::Error;
use crate::Hash;
use crate::Result;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use reqwest::Client;
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
    chunks: Chunks,
    #[cfg(feature = "progress")]
    pb: Option<indicatif::ProgressBar>,
}

impl Downloader {
    pub fn get_client(&self) -> &Client {
        &self.client
    }
    pub fn get_url(&self) -> String {
        self.url.to_string()
    }
    pub fn filename(&self) -> &str {
        &self.filename
    }
    async fn assemble_downloader(
        url: &str,
        workers: u8,
        length: u64,
        client: Client,
    ) -> Result<Self> {
        let parsed = reqwest::Url::parse(url)?;
        if length == 0 {
            return Err(Error::NoLen);
        }
        let chunks = Chunks::new(0, length - 1, length / workers as u64)?;
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
    pub async fn new_manual(url: &str, workers: u8, length: u64) -> Result<Self> {
        let client = Client::new();
        Self::assemble_downloader(url, workers, length, client).await
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
    ///     // If only one TLS feature is enabled
    ///     let downloader = Downloader::new("https://crates.io", 5).await?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(url: &str, workers: u8) -> Result<Self> {
        let client = Client::new();
        let length = content_length(&client, url).await?;
        Self::assemble_downloader(url, workers, length, client).await
    }
    /// Get filename from the url, returns an error if the url contains no filename
    ///
    /// # Arguments
    ///
    /// * `url` - &str with the url
    ///
    /// # Example
    /// ```
    /// use manic::Downloader;
    /// use manic::Error;
    /// # fn main() -> Result<(), Error> {
    ///     let url = manic::Url::parse("https://test.rs/test.zip")?;
    ///     let name = Downloader::get_filename(&url)?;
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
    /// Enable progress reporting
    #[cfg(feature = "progress")]
    pub fn progress_bar(&mut self) -> &mut Self {
        self.pb = Some(indicatif::ProgressBar::new(self.length));
        self
    }
    /// Set the progress bar style
    #[cfg(feature = "progress")]
    pub fn bar_style(&self, style: indicatif::ProgressStyle) {
        if let Some(pb) = &self.pb {
            pb.set_style(style);
        }
    }
    /// Add a SHA checksum to verify against
    /// # Arguments
    /// * `hash` - [`Hash`][crate::Hash] to verify against
    pub fn verify(&mut self, hash: Hash) -> Self {
        self.hash = Some(hash);
        self.to_owned()
    }
    #[instrument(skip(self))]
    async fn download_chunk(&self, val: String) -> Result<Vec<u8>> {
        let mut resp = self
            .client
            .get(&self.url.to_string())
            .header(RANGE, val)
            .send()
            .await?;
        {
            let mut res = Vec::new();
            while let Some(chunk) = resp.chunk().await? {
                #[cfg(feature = "progress")]
                if let Some(bar) = &self.pb {
                    bar.inc(chunk.len() as u64);
                }
                res.append(&mut chunk.to_vec());
            }
            Ok(res)
        }
    }
    /// Download the file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use manic::Downloader;
    /// use manic::Error;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// let client = Downloader::new("https://crates.io", 5).await?;
    /// let result = client.download().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self), fields(URL=%self.url, tasks=%self.workers))]
    pub async fn download(&self) -> Result<Vec<u8>> {
        let mb = &self.length / 1000000;
        debug!("File size: {}MB", mb);
        let hndl_vec = self
            .chunks
            .into_iter()
            .map(move |x| self.download_chunk(x))
            .collect::<Vec<_>>();
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
    /// Download and verify the file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use manic::Downloader;
    /// use manic::Error;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// let client = Downloader::new("https://crates.io", 5).await?;
    /// let result = client.download_and_verify().await?;
    /// # Ok(())
    /// # }
    /// ```
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
    /// Used to download, save to a file and verify against a SHA256 sum,
    /// returns an error if the connection fails or if the sum doesn't match the one provided
    ///
    /// # Arguments
    /// * `path` - path to save the file to, if it's a directory then the original filename is used
    /// * `verify` - set true to verify the file against the hash
    ///
    /// # Example
    ///
    /// ```no_run
    /// use manic::Downloader;
    /// use manic::Error;
    /// use manic::Hash;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    ///     let client = Downloader::new("https://crates.io", 5).await?.verify(hash);
    ///     client.download_and_save("~/Downloads", true).await?;
    ///     Ok(())
    ///  }
    /// ```
    ///
    #[instrument(skip(self))]
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
}

#[instrument(skip(client, url), fields(URL=%url))]
async fn content_length(client: &Client, url: &str) -> Result<u64> {
    let resp = client.head(url).send().await?;
    debug!("Response code: {}", resp.status());
    debug!("Received HEAD response: {:?}", resp.headers());
    let len = resp.headers().get("content-length").ok_or(Error::NoLen);
    if len.is_ok() && resp.status().is_success() {
        len?.to_str()
            .map_err(|_x| Error::NoLen)?
            .parse::<u64>()
            .map_err(|_x| Error::NoLen)
    } else {
        let resp = client.get(url).header(RANGE, "0-0").send().await?;
        debug!("Response code: {}", resp.status());
        debug!("Received GET 1B response: {:?}", resp.headers());
        resp.headers()
            .get(CONTENT_LENGTH)
            .ok_or(Error::NoLen)?
            .to_str()
            .map_err(|_| Error::NoLen)?
            .parse::<u64>()
            .map_err(|_| Error::NoLen)
    }
}
