#[cfg(feature = "progress")]
use indicatif::ProgressBar;
use tracing::debug;

use crate::chunk::Chunks;
use crate::Result;
use crate::{Client, Url};
use crate::{Error, Hash};
use reqwest::header::RANGE;
use std::convert::AsRef;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::instrument;

#[derive(Debug, Clone)]
pub enum WriteType {
    File(PathBuf),
    Mem,
}

#[derive(Debug, Clone)]
pub struct File {
    pub filename: String,
    pub url: String,
    length: u64,
    pub workers: u8,
    pub chunks: Chunks,
    writer: WriteType,
    pub hash: Option<Hash>,
    #[cfg(feature = "progress")]
    pub bar: Option<ProgressBar>,
}

impl File {
    pub async fn new(url: &str, workers: u8, client: &Client) -> Result<Self> {
        let length = content_length(client, url).await?;
        Self::init(url, length, workers)
    }
    pub fn init(url: &str, length: u64, workers: u8) -> Result<Self> {
        let chunks = Chunks::new(0, length - 1, length / workers as u64)?;
        let filename = Self::get_filename(url)?;
        Ok(Self {
            filename,
            url: url.to_string(),
            length,
            workers,
            chunks,
            writer: WriteType::Mem,
            hash: None,
            #[cfg(feature = "progress")]
            bar: None,
        })
    }
    pub fn save_to_file(&mut self, path: impl AsRef<Path>) {
        let file = path.as_ref().to_path_buf().join(Path::new(&self.filename));
        self.writer = WriteType::File { 0: file };
    }

    pub fn get_filename(url: &str) -> Result<String> {
        let parsed = Url::parse(url)?;
        parsed
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
    /// Enable progress reporting
    #[cfg(feature = "progress")]
    pub fn progress_bar(&mut self, bar: ProgressBar) -> &mut Self {
        self.bar = Some(bar);
        self
    }

    /// Set the progress bar style
    #[cfg(feature = "progress")]
    pub fn bar_style(&self, style: indicatif::ProgressStyle) {
        if let Some(pb) = &self.bar {
            pb.set_style(style);
        }
    }
    /// Add a SHA checksum to verify against
    /// # Arguments
    /// * `hash` - [`Hash`][crate::Hash] to verify against
    pub fn verify(&mut self, hash: Hash) {
        self.hash = Some(hash);
    }
    pub fn get_length(&self) -> u64 {
        self.length
    }

    #[instrument(skip(self, client))]
    pub async fn download_chunk(&self, val: String, client: &Client) -> Result<Vec<u8>> {
        let mut resp = client.get(&self.url).header(RANGE, val).send().await?;
        {
            let mut res = Vec::new();
            while let Some(chunk) = resp.chunk().await? {
                #[cfg(feature = "progress")]
                if let Some(bar) = &self.bar {
                    bar.inc(chunk.len() as u64);
                }
                res.append(&mut chunk.to_vec());
            }
            Ok(res)
        }
    }

    #[instrument(skip(self), fields(URL=%self.url, tasks=%self.workers))]
    async fn download_inner(&self, client: &Client) -> Result<Vec<u8>> {
        let mb = self.get_length() / 1000000;
        debug!("File size: {}MB", mb);
        let hndl_vec = self
            .chunks
            .into_iter()
            .map(move |x| self.download_chunk(x, client))
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
    #[instrument(skip(self))]
    async fn download_and_verify(&self, client: &Client) -> Result<Vec<u8>> {
        let data = self.download_inner(client).await?;
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
    /// use manic::File;
    /// use manic::Error;
    /// use manic::Hash;
    /// use reqwest::Client;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    ///     let req = Client::new();
    ///     let client = File::new("https://crates.io", 5, &req).await?.verify(hash);
    ///     client.download(&req).await?;
    ///     Ok(())
    ///  }
    /// ```
    ///
    #[instrument(skip(self))]
    pub async fn download(&self, client: &Client) -> Result<Option<Vec<u8>>> {
        let data = self.download_and_verify(client).await?;
        match &self.writer {
            WriteType::File(path) => {
                let mut file = fs::File::create(path).await?;
                file.write_all(data.as_slice()).await?;
                file.sync_all().await?;
                file.flush().await?;
            }
            WriteType::Mem => return Ok(Some(data)),
        }
        Ok(None)
    }
}

async fn content_length(client: &Client, url: &str) -> Result<u64> {
    let resp = client.head(url).send().await?;
    debug!("Response code: {}", resp.status());
    debug!("Received HEAD response: {:?}", resp.headers());
    resp.content_length().ok_or(Error::NoLen)
}
