use crate::chunk::{self, Chunks};
use crate::utils::*;
use crate::Connector;
use crate::Hash;
use crate::Result;
use hyper::client::connect::Connect;
use hyper::Client;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, instrument};

/// Main type of the crate, use to download the file and optionally verify it
#[derive(Debug)]
pub struct Downloader<C>
where
    C: Connector + Connect,
{
    client: Client<C>,
    hash: Option<Hash>,
    chunks: Chunks,
    workers: u8,
    url: String,
    length: u64,
    verify: bool,
    #[cfg(feature = "progress")]
    bar: Option<indicatif::ProgressBar>,
}

impl<C> Downloader<C>
where
    C: Connector + Connect,
{
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
    /// // On both features enabled
    /// # #[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
    /// use manic::{Rustls, OpenSSL};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), manic::Error> {
    ///     // If only one TLS feature is enabled
    ///     # #[cfg(any(all(feature = "rustls-tls", not(feature = "openssl-tls")), all(feature = "openssl-tls", not(feature = "rustls-tls"))))]
    ///     let client = Downloader::new("https://crates.io", 5).await?;
    ///
    ///     // With Rustls
    ///     # #[cfg(all(feature = "openssl-tls", feature = "rustls-tls"))]
    ///     let client = Downloader::<Rustls>::new("https://crates.io", 5).await?;
    ///
    ///     # #[cfg(all(feature = "openssl-tls", feature = "rustls-tls"))]
    ///     // With OpenSSL
    ///     let client = Downloader::<OpenSSL>::new("https://crates.io", 5).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(url: &str, workers: u8) -> Result<Self> {
        let connector = C::new();
        let client = Client::builder().build::<_, hyper::Body>(connector);
        let redirect = check_redirects(&client, url).await?;
        let url = if let Some(new_url) = &redirect {
            new_url
        } else {
            url
        };
        let len = get_length(&client, url).await?;
        let chunks = Chunks::new(0, len - 1, (len / workers as u64) as u32)?;
        #[cfg(not(feature = "progress"))]
        return Ok(Self {
            client,
            hash: None,
            chunks,
            workers,
            url: url.to_string(),
            length: len,
            verify: false,
        });
        #[cfg(feature = "progress")]
        return Ok(Self {
            client,
            hash: None,
            chunks,
            workers,
            url: url.to_string(),
            length: len,
            verify: false,
            bar: None,
        });
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
        if let Some(sha) = self.get_hash() {
            compare_sha(sha, &data)?;
            debug!("Compared");
        }
        Ok(data)
    }

    /// Used to download, save to a file and verify against a SHA256 sum,
    /// returns an error if the connection fails or if the sum doesn't match the one provided
    ///
    /// # Arguments
    /// * `path` - where to download the file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "rustls-tls")]
    /// # use manic::Rustls;
    /// # use manic::downloader::Downloader;
    /// # use manic::Error;
    /// # use manic::Hash;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    /// # let mut dl = Downloader::<Rustls>::new("https://crates.io", 5).await?;
    /// dl.verify(hash);
    /// dl.download_and_save("~/Downloads/").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_and_save(&self, path: &str) -> Result<()> {
        let mut result = {
            let name = get_filename(self.get_url())?;
            let file_path = Path::new(path).join(name);
            File::create(file_path).await?
        };
        let data = if self.verifiable() {
            self.download_and_verify().await?
        } else {
            self.download().await?
        };
        result.write_all(data.as_slice()).await?;
        result.sync_all().await?;
        result.flush().await?;
        Ok(())
    }
    fn get_hash(&self) -> &Option<Hash> {
        &self.hash
    }
    /// Download the file
    ///
    /// # Example
    /// ```
    /// # #[cfg(feature = "rustls-tls")]
    /// # use manic::Rustls;
    /// # #[cfg(feature = "openssl-tls")]
    /// # use manic::downloader::Downloader;
    /// # use manic::Error;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// # use manic::OpenSSL;
    /// # #[cfg(feature = "rustls-tls")]
    /// # let dl = Downloader::<Rustls>::new("https://crates.io", 5).await?;
    /// # #[cfg(feature = "openssl-tls")]
    /// # let dl = Downloader::<OpenSSL>::new("https://crates.io", 5).await?;
    /// let result = dl.download().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub async fn download(&self) -> Result<Vec<u8>> {
        let mb = self.length / 1000000;
        debug!("File size: {}MB", mb);
        #[cfg(not(feature = "progress"))]
        let hndl_vec = self
            .chunks
            .into_iter()
            .map(move |x| chunk::download(x, &self.url, &self.client))
            .collect::<Vec<_>>();
        #[cfg(feature = "progress")]
        let hndl_vec = self
            .chunks
            .into_iter()
            .map(move |x| chunk::download(x, &self.url, &self.client, &self.bar))
            .collect::<Vec<_>>();
        let data = collect_results(hndl_vec).await?;
        Ok(data)
    }
    fn verifiable(&self) -> bool {
        self.verify
    }
    fn get_url(&self) -> &str {
        &self.url
    }
    /// Set the hash to verify against
    pub fn verify(&mut self, hash: Hash) {
        self.hash = Some(hash);
        self.verify = true;
    }
}
