use std::path::Path;

use futures::StreamExt;
use tracing::{debug, instrument};

use crate::chunk::{ChunkVec, Chunks};
use crate::Client;
use crate::Hash;
use crate::ManicError;
use crate::Result;
use hyper::Uri;
use indicatif::ProgressBar;

/// Main type of the crate, use to download the file and optionally verify it
#[derive(Debug, Clone)]
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
    async fn assemble(client: Client, url: &str, workers: u8, size: u64) -> Result<Self> {
        let uri = url.parse::<Uri>()?;
        let filename = Self::split_filename(&uri)?;
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
    pub(crate) fn get_len(&self) -> u64 {
        self.length
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
        let client = Client::new()?;
        let len = client.content_length(url).await?;
        debug!("Len: {}", len);
        Self::assemble(client, url, workers, len).await
    }
    /// Get filename from the url, returns an error if the url contains no filename
    #[instrument(skip(url),fields(URL=%url))]
    fn split_filename(url: &Uri) -> Result<String> {
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
            .ok_or_else(|| ManicError::NoFilename("No filename".to_string()))
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

    #[cfg(feature = "progress")]
    pub(crate) fn connect_progress(&mut self, to_add: ProgressBar) {
        self.bar = Some(to_add);
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
    /// # use manic::Downloader;
    /// # use manic::ManicError;
    /// # use manic::Hash;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), ManicError> {
    /// let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    /// # let mut dl = Downloader::new("https://crates.io", 5).await?;
    /// dl.verify(hash);
    /// dl.download_and_save("~/Downloads/").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_and_save(&self, path: &str) -> Result<()> {
        let original_path = Path::new(path);
        let file_path = if original_path.is_dir() {
            original_path.join(&self.filename)
        } else {
            original_path.to_path_buf()
        };
        let data = self.download().await?;
        data.save_to_file(file_path).await
    }
    /// Download the file
    ///
    /// # Example
    /// ```no_run
    /// # use manic::Downloader;
    /// # use manic::ManicError;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), ManicError> {
    /// # let dl = Downloader::new("https://crates.io", 5).await?;
    /// let result = dl.download().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub async fn download(&self) -> Result<ChunkVec> {
        let mb = self.length / 1000000;
        debug!("File size: {}MB", mb);
        let res = self
            .chunks
            .download(
                self.client.clone(),
                self.url.to_string(),
                #[cfg(feature = "progress")]
                self.bar.clone(),
            )
            .await?;
        if let Some(h) = &self.hash {
            res.verify(h).await?;
        }
        Ok(res)
    }
    /// Set the hash to verify against
    pub fn verify(&mut self, hash: Hash) -> &mut Self {
        self.hash = Some(hash);
        self
    }
}
