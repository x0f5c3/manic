#![allow(dead_code)]
use super::chunk::{ChunkVec, Chunks};
use super::error::ManicError;
use super::error::Result;
use super::hash::Hash;
use super::multi::Downloaded;
use indicatif::ProgressBar;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_LENGTH, RANGE};
use std::fs::File;
use std::io::Write;
use std::path::Path;
// use tokio::task::JoinHandle;
use rayon::prelude::*;
use std::thread::JoinHandle;
use tracing::{debug, instrument};

#[derive(Debug, Clone, Builder)]
pub struct Downloader {
    filename: String,
    #[builder(default, setter(skip))]
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
    pub fn get_len(&self) -> u64 {
        self.length
    }
    pub fn filename(&self) -> &str {
        &self.filename
    }
    fn assemble_downloader(url: &str, workers: u8, length: u64, client: Client) -> Result<Self> {
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
    pub fn new_manual(url: &str, workers: u8, length: u64) -> Result<Self> {
        let client = Client::new();
        Self::assemble_downloader(url, workers, length, client)
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
    /// # fn main() -> Result<(), manic::ManicError> {
    ///     // If only one TLS feature is enabled
    ///     let downloader = Downloader::new("https://crates.io", 5)?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(url: &str, workers: u8) -> Result<Self> {
        let client = Client::new();
        let length = content_length(&client, url)?;
        Self::assemble_downloader(url, workers, length, client)
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
        self.pb = Some(indicatif::ProgressBar::new(self.length));
        self
    }
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
    /// use manic::Downloader;
    /// use manic::ManicError;
    /// # #[tokio::main]
    /// # fn main() -> Result<(), ManicError> {
    /// let client = Downloader::new("https://crates.io", 5)?;
    /// let result = client.download()?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self), fields(URL=%self.url, tasks=%self.workers))]
    pub fn download(&self) -> Result<ChunkVec> {
        let mb = &self.length / 1000000;
        debug!("File size: {}MB", mb);
        let chnks = self.chunks;
        let url = self.url.clone();
        let client = self.client.clone();
        #[cfg(feature = "progress")]
        let pb = self.pb.clone();
        let result = chnks.download(client, url.to_string(), pb)?;
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
    /// use manic::Downloader;
    /// use manic::ManicError;
    /// use manic::Hash;
    /// #[tokio::main]
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
        data.save(c)?;
        result.sync_all()?;
        result.flush()?;
        Ok(())
    }
}

#[instrument(skip(client, url), fields(URL=%url))]
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

pub fn join_all<T: Clone + Send>(i: Vec<JoinHandle<Result<T>>>) -> Result<Vec<T>> {
    let (successful, errs): (Vec<Result<T>>, Vec<Result<T>>) = i
        .into_par_iter()
        .map(|x| x.join())
        .filter_map(|x| x.ok())
        .partition(|x| x.is_ok());
    check_err(
        errs.into_par_iter().filter_map(|x| x.err()).collect(),
        successful.into_par_iter().filter_map(|x| x.ok()).collect(),
    )
}
pub fn check_err<T: Clone>(err: Vec<ManicError>, good: Vec<T>) -> Result<Vec<T>> {
    if !err.is_empty() && good.is_empty() {
        Err(err.into())
    } else if !good.is_empty() {
        Ok(good)
    } else {
        Err(ManicError::NoResults)
    }
}
