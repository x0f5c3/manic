#[cfg(feature = "github")]
use crate::github::Asset;
use crate::Client;
use crate::Error;
use crate::Hash;
use crate::Result;
use reqwest::header::{HeaderMap, USER_AGENT};
use tracing::instrument;
use crate::file::File;


#[cfg(feature = "progress")]
use indicatif::MultiProgress;
use std::path::Path;
use indicatif::ProgressBar;
use futures::StreamExt;
use std::future::Future;


#[derive(Debug)]
pub struct Downloader {
    client: Client,
    files: Vec<File>,
    #[cfg(feature = "progress")]
    pb: Option<MultiProgress>,
    #[cfg(feature = "progress")]
    pbs: Option<Vec<ProgressBar>>,
}

impl Downloader {
    #[cfg(feature = "github")]
    /// Assemble a Downloader from a GitHub asset
    pub fn new_from_asset(asset: Asset, workers: u8) -> Result<Self> {
        let len = asset.size as u64;
        let url = asset.browser_download_url;
        let mut heads = HeaderMap::new();
        heads.insert(USER_AGENT, "Manic_DL".parse()?);
        let client = Client::builder().default_headers(heads).build()?;
        let file = File::init(&url, len, workers)?;
        Self::assemble_downloader(file, client)
    }

    fn assemble_downloader(
        file: File,
        client: Client,
    ) -> Result<Self> {
        let files = vec!(file);
        return Ok(Self {
            client,
            files,
            #[cfg(feature = "progress")]
            pb: None,
            #[cfg(feature = "progress")]
            pbs: None,
        });
    }

    /// Assemble the downloader manually in case the server doesn't allow head requests
    pub fn new_manual(url: &str, workers: u8, length: u64) -> Result<Self> {
        let client = Client::new();
        let file = File::init(url, length, workers)?;
        Self::assemble_downloader(file, client)
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
    #[instrument]
    pub async fn new(url: &str, workers: u8) -> Result<Self> {
        let client = Client::new();
        let file = File::new(url, workers, &client).await?;
        Self::assemble_downloader(file, client)
    }
    #[instrument]
    pub async fn add_to_queue(&mut self, url: &str, workers: u8) -> Result<()> {
        let file = File::new(url, workers, &self.client).await?;
        self.files.push(file);
        Ok(())
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
    ///     let name = Downloader::get_filename("http://test.rs/test.zip")?;
    ///     assert_eq!("test.zip", name);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_filename(url: &str) -> Result<String> {
        let parsed = reqwest::Url::parse(url)?;
        parsed.path_segments()
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
    pub fn progress_bar(&mut self) {
        let mpb = MultiProgress::new();
        let mut pbs: Vec<ProgressBar> = Vec::new();
        for file in &mut self.files {
            let pb = mpb.add(ProgressBar::new(file.get_length()));
            file.progress_bar(pb.clone());
            pbs.push(pb);
        }
        self.pb = Some(mpb);
        self.pbs = Some(pbs);
    }

    /// Set the progress bar style
    #[cfg(feature = "progress")]
    pub fn bar_style(&self, style: indicatif::ProgressStyle) {
        if let Some(pbs) = &self.pbs {
            for i in pbs {
                i.set_style(style.clone());
            }
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
    #[instrument(skip(self))]
    pub async fn download(mut self) -> Result<Vec<Option<Vec<u8>>>> {
        let mut fut_vec = Vec::new();
        for f in self.files {
            fut_vec.push(f.download(&self.client).await?);
        }
        Ok(fut_vec)
    }
    /// Set the file to save on disk
    ///
    /// # Arguments
    /// * `path` - directory where the file will be saved
    pub fn on_disk(&mut self, path: impl AsRef<Path> + Clone) -> Result<()> {
        for i in &mut self.files {
            i.save_to_file(path.clone());
        }
        Ok(())
    }

}

