#![allow(dead_code)]

use super::chunk::{ChunkVec, Chunks};
use super::multi::Downloaded;
use crate::Hash;
use crate::{ManicError, Result};
#[cfg(feature = "progress")]
use indicatif::ProgressBar;
use rayon::prelude::*;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use rusty_pool::JoinHandle;
use rusty_pool::ThreadPool;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tracing::{debug, instrument};

#[derive(Clone, Builder)]
pub struct Downloader {
    filename: String,
    #[builder(default, setter(skip))]
    client: Client,
    workers: u8,
    url: reqwest::Url,
    hash: Option<Hash>,
    length: u64,
    chunks: Chunks,
    pool: ThreadPool,
    #[cfg(feature = "progress")]
    pb: Option<ProgressBar>,
}

impl Downloader {
    pub fn get_client(&self) -> &Client {
        &self.client
    }
    pub fn get_url(&self) -> String {
        self.url.to_string()
    }
    pub fn get_len(&self) -> u64 {
        self.length
    }
    pub fn filename(&self) -> &str {
        &self.filename
    }
    pub(crate) fn new_multi(url: &str, workers: u8, pool: ThreadPool) -> Result<Self> {
        let client = Client::new();
        let length = content_length(&client, url)?;
        Self::assemble_downloader(url, workers, length, client, pool)
    }
    fn assemble_downloader(
        url: &str,
        workers: u8,
        length: u64,
        client: Client,
        pool: ThreadPool,
    ) -> Result<Self> {
        let parsed = reqwest::Url::parse(url)?;
        if length == 0 {
            return Err(ManicError::NoLen);
        }
        let chunks = Chunks::new(0, length - 1, length / workers as u64)?;
        let filename = Self::url_to_filename(&parsed)?;
        #[cfg(not(feature = "progress"))]
        return Ok(Self {
            filename,
            client,
            workers,
            url: parsed,
            hash: None,
            length,
            chunks,
            pool,
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
            pool,
        });
    }
    pub fn new_manual(url: &str, workers: u8, length: u64) -> Result<Self> {
        let client = Client::new();
        let pool = rusty_pool::Builder::new()
            .max_size(workers as usize)
            .build();
        Self::assemble_downloader(url, workers, length, client, pool)
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
    /// use manic::threaded::Downloader;
    /// # fn main() -> Result<(), manic::ManicError> {
    ///    let downloader = Downloader::new("https://crates.io", 5)?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(url: &str, workers: u8) -> Result<Self> {
        let client = Client::new();
        let length = content_length(&client, url)?;
        let pool = rusty_pool::Builder::new()
            .max_size(workers as usize)
            .build();
        Self::assemble_downloader(url, workers, length, client, pool)
    }
    pub fn url_to_filename(url: &reqwest::Url) -> Result<String> {
        url.path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| {
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            })
            .ok_or_else(|| ManicError::NoFilename(url.to_string()))
    }
    /// Enable progress reporting
    #[cfg(feature = "progress")]
    pub fn progress_bar(&mut self) -> &mut Self {
        self.pb = Some(ProgressBar::new(self.length));
        self
    }
    /// Connect the `ProgressBar`[ProgressBar] to the `MultiProgress`[indicatif::MultiProgress]
    #[cfg(feature = "progress")]
    pub fn connect_progress(&mut self, pb: ProgressBar) {
        self.pb = Some(pb);
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
    /// Download the file and verify if hash is set
    ///
    /// # Example
    ///
    /// ```no_run
    /// use manic::threaded::Downloader;
    /// use manic::ManicError;
    /// # fn main() -> Result<(), ManicError> {
    /// let client = Downloader::new("https://crates.io", 5)?;
    /// let result = client.download()?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self), fields(URL = % self.url, tasks = % self.workers))]
    pub fn download(&self) -> Result<ChunkVec> {
        let mb = &self.length / 1000000;
        debug!("File size: {}MB", mb);
        let chnks = self.chunks;
        let url = self.url.clone();
        let client = self.client.clone();
        #[cfg(feature = "progress")]
        let pb = self.pb.clone();
        let result = chnks.download(
            client,
            url.to_string(),
            #[cfg(feature = "progress")]
            pb,
            self.pool.clone(),
        )?;
        if let Some(hash) = &self.hash {
            result.verify(hash.clone())?;
            debug!("Compared");
        }
        Ok(result)
    }
    pub fn multi_download(self) -> Result<Downloaded> {
        let res = self.download()?;
        Ok(Downloaded::new(self.get_url(), self.filename, res))
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
    /// use manic::threaded::Downloader;
    /// use manic::ManicError;
    /// use manic::Hash;
    /// fn main() -> Result<(), ManicError> {
    ///     let hash = Hash::new_sha256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    ///     let client = Downloader::new("https://crates.io", 5)?.verify(hash);
    ///     client.download_and_save("~/Downloads")?;
    ///     Ok(())
    ///  }
    /// ```
    ///
    #[instrument(skip(self))]
    pub fn download_and_save(&self, path: &str) -> Result<()> {
        let mut result = {
            let original_path = Path::new(path);
            let file_path = if original_path.is_dir() {
                original_path.join(&self.filename)
            } else {
                original_path.to_path_buf()
            };
            File::create(file_path)?
        };
        let data = self.download()?;
        let c = result.try_clone()?;
        data.save(c, self.pool.clone())?;
        result.sync_all()?;
        result.flush()?;
        Ok(())
    }
}

#[instrument(skip(client, url), fields(URL = % url))]
fn content_length(client: &Client, url: &str) -> Result<u64> {
    let resp = client.head(url).send()?;
    debug!("Response code: {}", resp.status());
    debug!("Received HEAD response: {:?}", resp.headers());
    let len = resp
        .headers()
        .get("content-length")
        .ok_or(ManicError::NoLen);
    if len.is_ok() && resp.status().is_success() {
        len?.to_str()
            .map_err(|_x| ManicError::NoLen)?
            .parse::<u64>()
            .map_err(|e| e.into())
    } else {
        let resp = client.get(url).header(RANGE, "0-0").send()?;
        debug!("Response code: {}", resp.status());
        debug!("Received GET 1B response: {:?}", resp.headers());
        resp.headers()
            .get(CONTENT_LENGTH)
            .ok_or(ManicError::NoLen)?
            .to_str()?
            .parse::<u64>()
            .map_err(|e| e.into())
    }
}

pub(crate) fn join_all<T: Clone + Send>(i: Vec<JoinHandle<Result<T>>>) -> Result<Vec<T>> {
    i.into_par_iter()
        .map(|x| x.try_await_complete().map_err(ManicError::Canceled))
        .collect::<Result<Vec<Result<T>>>>()?
        .into_par_iter()
        .collect()
}
