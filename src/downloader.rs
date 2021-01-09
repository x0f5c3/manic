use crate::chunk::{self, Chunks};
use crate::ClientExt;
use crate::Hash;
use crate::Result;
use crate::{Connector, Error};
use futures::Future;
use hyper::client::connect::Connect;
use hyper::Client;
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, instrument};

/// Main type of the crate, use to download the file and optionally verify it
#[derive(Debug)]
pub struct Downloader<C>
where
    C: Connect + Clone + Send + Sync + Unpin + 'static,
{
    client: Client<C>,
    hash: Option<Hash>,
    chunks: Chunks,
    workers: u8,
    url: hyper::Uri,
    length: u64,
    verify: bool,
    #[cfg(feature = "progress")]
    bar: Option<indicatif::ProgressBar>,
}
impl<C> Downloader<C>
where
    C: Connector + Connect + Unpin + Clone + Send + Sync + 'static,
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
        let uri = url.parse::<hyper::Uri>()?;
        let redirect = client.check_redirects(uri).await?;
        let len = client.content_length(&redirect).await?;
        let chunks = Chunks::new(0, len - 1, (len / workers as u64) as u32)?;
        #[cfg(not(feature = "progress"))]
        return Ok(Self {
            client,
            hash: None,
            chunks,
            workers,
            url: redirect,
            length: len,
            verify: false,
        });
        #[cfg(feature = "progress")]
        return Ok(Self {
            client,
            hash: None,
            chunks,
            workers,
            url: redirect,
            length: len,
            verify: false,
            bar: None,
        });
    }
}

impl<C> Downloader<C>
where
    C: Connect + Unpin + Send + Sync + Clone + 'static,
{
    /// Compare SHA256 of the data to the given sum,
    /// will return an error if the sum is not equal to the data's
    /// # Arguments
    /// * `data` - u8 slice of data to compare
    /// * `hash` - SHA256 sum to compare to
    ///
    /// # Example
    ///
    /// ```
    /// use manic::utils::compare_sha;
    /// use manic::Error;
    /// use manic::Hash;
    /// # fn main() -> Result<(), Error> {
    ///     let data: &[u8] = &[1,2,3];
    ///     let hash = Hash::SHA256("039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81".to_string());
    ///     compare_sha(&hash,data).unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(data, hash), fields(SHA=%hash))]
    pub fn compare_sha(hash: &Hash, data: &[u8]) -> Result<()> {
        let hashed = format!("{}", hash);
        debug!("Comparing sum {}", hashed);
        let hexed = match hash {
            Hash::SHA256(_) => format!("{:x}", Sha256::digest(data)),
            Hash::SHA224(_) => format!("{:x}", Sha224::digest(data)),
            Hash::SHA512(_) => format!("{:x}", Sha512::digest(data)),
            Hash::SHA384(_) => format!("{:x}", Sha384::digest(data)),
        };
        debug!("SHA256 sum: {}", hexed);
        if hexed == hashed {
            debug!("SHA256 MATCH!");
            Ok(())
        } else {
            Err(Error::SHA256MisMatch(hexed))
        }
    }
    /// Get filename from the url, returns an error if the url contains no filename
    #[instrument(skip(self), fields(URL=%self.url))]
    pub(crate) fn get_filename(&self) -> Result<String> {
        self.url
            .path()
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
            Self::compare_sha(sha, &data)?;
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
            let name = self.get_filename()?;
            let file_path = Path::new(path).join(name);
            File::create(file_path).await?
        };
        let data = if self.hash.is_some() {
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
        self.hash.is_some()
    }
    /// Set the hash to verify against
    pub fn verify(&mut self, hash: Hash) {
        self.hash = Some(hash);
        self.verify = true;
    }
}

pub(crate) async fn collect_results(
    handle_vec: Vec<impl Future<Output = Result<Vec<u8>>>>,
) -> Result<Vec<u8>> {
    let data = {
        let mut result = Vec::new();
        for i in handle_vec {
            let mut curr_part = i
                .await
                .map_err(|_| Error::SHA256MisMatch("Failed".to_string()))?;
            result.append(&mut curr_part);
        }
        result
    };
    Ok(data)
}
