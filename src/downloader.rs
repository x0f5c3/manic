use crate::chunk::{self, Chunks};
use crate::Connector;
use crate::Error;
use hyper::Client;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, instrument};
use hyper::client::connect::Connect;
use crate::utils::*;
use crate::Hash;

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
    /// # Example
    ///
    /// ```rust
    /// use manic::downloader::Downloader;
    /// #[cfg(feature = "rustls-tls")]
    /// use manic::Rustls as TLS;
    /// #[cfg(feature = "openssl-tls")]
    /// use manic::OpenSSL as TLS;
    ///
    /// ##[tokio::main]
    /// # async fn main() -> Result<(), manic::Error> {
    ///     let dl = Downloader::<TLS>::new("https://crates.io", 5).await?;
    /// #     Ok(())
    /// # }
    /// ```
    pub async fn new(url: &str, workers: u8) -> Result<Self, Error> {
        let connector = C::new();
        let client = Client::builder().build::<_, hyper::Body>(connector);
        let len = get_length(&client, url).await?;
        let chunks = Chunks::new(0, len - 1, (len / workers as u64) as u32)?;
        Ok(Self {
            client,
            hash: None,
            chunks,
            workers,
            url: url.to_string(),
            length: len,
            verify: false,
        })
    }
    /// Set the SHA256 hash to check against
    pub fn verify(mut self, hash: Hash) -> Self {
        self.hash = Some(hash);
        self.verify = true;
        self
    }
    /// Download the file
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[cfg(feature = "rustls-tls")]
    /// use manic::Rustls as Conn;
    /// #[cfg(feature = "openssl-tls")]
    /// use manic::OpenSSL as Conn;
    /// use manic::downloader::Downloader;
    /// use manic::Error;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// let dl = Downloader::<Conn>::new("https://crates.io", 5).await?;
    /// let result = dl.download().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self), fields(URL=%self.url, tasks=%self.workers))]
    pub async fn download(&self) -> Result<Vec<u8>, Error> {
        let mb = self.length / 1000000;
        debug!("File size: {}MB", mb);
        let hndl_vec = self
            .chunks
            .into_iter()
            .map(move |x| chunk::download(x, &self.url, &self.client))
            .collect::<Vec<_>>();
        let data = collect_results(hndl_vec).await?;
        Ok(data)
    }
    /// Used to download and verify against a SHA256 sum,
    /// returns an error if the connection fails or if the sum doesn't match the one provided
    ///
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[cfg(feature = "rustls-tls")]
    /// use manic::Rustls as Conn;
    /// #[cfg(feature = "openssl-tls")]
    /// use manic::OpenSSL as Conn;
    /// use manic::downloader::Downloader;
    /// use manic::Error;
    /// # use manic::Hash;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// # let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    /// let dl = Downloader::<Conn>::new("https://crates.io", 5).await?.verify(hash);
    ///
    /// let data = dl.download_and_verify().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    #[instrument(skip(self))]
    pub async fn download_and_verify(&self) -> Result<Vec<u8>, Error> {
        let data = self.download().await?;
        debug!("Downloaded");
        if let Some(sha) = &self.hash {
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
    /// #[cfg(feature = "rustls-tls")]
    /// use manic::Rustls as Conn;
    /// #[cfg(feature = "openssl-tls")]
    /// use manic::OpenSSL as Conn;
    /// use manic::downloader::Downloader;
    /// use manic::Error;
    /// use manic::Hash;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    ///     let dl = Downloader::<Conn>::new("https://crates.io", 5).await?.verify(hash);
    ///     dl.download_and_save("~/Downloads/").await?;
    ///     Ok(())
    ///  }
    /// ```
    ///
    #[instrument(skip(self))]
    pub async fn download_and_save(&self, path: &str) -> Result<(), Error>{
        let mut result = {
            let name = get_filename(&self.url)?;
            let file_path = Path::new(path).join(name);
            File::create(file_path).await?
        };
        let data = if self.verify {
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


