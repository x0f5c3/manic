use crate::chunk::Chunks;
use crate::progress;
use crate::Connector;
use crate::Error;
use hyper::header::CONTENT_LENGTH;
use hyper::Client;
use indicatif::ProgressBar;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, instrument};

#[derive(Debug)]
pub struct Downloader<C>
where
    C: Connector,
{
    client: Client<C>,
    hash: Option<String>,
    chunks: Chunks,
    workers: u8,
    url: String,
    length: u64,
    verify: bool,
    bar: indicatif::ProgressBar,
}

impl<C> Downloader<C>
where
    C: Connector,
{
    pub async fn new(url: &str, workers: u8) -> Result<Self, Error> {
        let connector = C::new();
        let client = Client::builder().build::<_, hyper::Body>(connector);
        let len = Self::get_length(&client, url).await?;
        let chunks = Chunks::new(0, len - 1, (len / workers as u64) as u32)?;
        let pb = ProgressBar::new(len);
        Ok(Self {
            client,
            hash: None,
            chunks,
            workers,
            url: url.to_string(),
            length: len,
            verify: false,
            bar: pb,
        })
    }
    pub fn to_verify(mut self, hash: &str) -> Self {
        self.hash = Some(hash.to_string());
        self.verify = true;
        self
    }
    pub fn bar_style(self, style: indicatif::ProgressStyle) -> Self {
        self.bar.set_style(style);
        self
    }
    /// Get the content-length header using a head request
    ///
    /// # Arguments
    ///
    /// * `url` - &str with the url
    /// * `client` - optional reference to a reqwest [`Client`][reqwest::Client] in case custom settings are needed
    ///
    /// # Example
    ///
    ///```no_run
    /// use manic::downloader;
    /// use manic::Error;
    /// use reqwest::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    ///     let client = Client::new();
    ///     let length = downloader::get_length("https://docs.rs", Some(&client)).await.unwrap();
    ///     assert_eq!(25853, length);
    /// #   Ok(())
    /// # }
    /// ```
    #[instrument(skip(client),fields(URL=%url))]
    pub async fn get_length(client: &Client<C>, url: &str) -> Result<u64, Error> {
        let req = hyper::Request::head(url)
            .body(hyper::Body::empty())
            .map_err(|e| Error::REQError(e))?;
        let head_req = client.request(req.into()).await?;
        let headers = head_req.headers();
        debug!("Received head response: {:?}", headers);
        headers[CONTENT_LENGTH]
            .to_str()?
            .parse::<u64>()
            .map_err(Into::into)
    }
    /// Get filename from the url, returns an error if the url contains no filename
    ///
    /// # Arguments
    ///
    /// * `url` - &str with the url
    ///
    /// # Example
    /// ```
    /// use manic::downloader;
    /// use manic::Error;
    /// # fn main() -> Result<(), Error> {
    ///     let name = downloader::get_filename("http://test.rs/test.zip")?;
    ///     assert_eq!("test.zip", name);
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub fn get_filename(&self) -> Result<String, Error> {
        let parsed_url = self.url.parse::<hyper::Uri>()?;
        parsed_url
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
    /// Download the file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use reqwest::Client;
    /// use manic::downloader;
    /// use manic::Error;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// let client = Client::new();
    /// let result = downloader::download(&client, "https://crates.io", 5).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download(&self) -> Result<Vec<u8>, Error> {
        let mb = self.length / 1000000;
        debug!("File size: {}MB", mb);
        let hndl_vec = self
            .chunks
            .into_iter()
            .map(move |x| progress::download_chunk(x, &self.url, &self.client, &self.bar))
            .collect::<Vec<_>>();
        let data = {
            let mut result = Vec::new();
            for i in hndl_vec {
                let mut curr_part = i
                    .await
                    .map_err(|_| Error::SHA256MisMatch("Failed".to_string()))?;
                result.append(&mut curr_part);
            }
            result
        };

        Ok(data)
    }
    /// Used to download and verify against a SHA256 sum,
    /// returns an error if the connection fails or if the sum doesn't match the one provided
    ///
    /// # Arguments
    /// * `client` - reference to a reqwest [`Client`][reqwest::Client]
    /// * `url` - &str with the url
    /// * `workers` - amount of concurrent downloads
    /// * `hash` - SHA256 sum to compare to
    ///
    /// # Example
    ///
    /// ```no_run
    /// use reqwest::Client;
    /// use manic::downloader;
    /// use manic::Error;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Error> {
    /// # let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81";
    /// let client = Client::new();
    /// let data = downloader::download_and_verify(&client,"https://crates.io", 5, hash).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    #[instrument(skip(self))]
    pub async fn download_and_verify(&self) -> Result<Vec<u8>, Error> {
        let data = self.download().await?;
        debug!("Downloaded");
        if let Some(sha) = &self.hash {
            crate::downloader::compare_sha(&sha, &data)?;
            debug!("Compared");
        }
        Ok(data)
    }

    /// Used to download, save to a file and verify against a SHA256 sum,
    /// returns an error if the connection fails or if the sum doesn't match the one provided
    ///
    /// # Arguments
    /// * `client` - reference to a reqwest [`Client`][reqwest::Client]
    /// * `url` - &str with the url
    /// * `workers` - amount of concurrent downloads
    /// * `hash` - optional SHA256 sum to compare to
    /// * `path` - where to download the file
    /// * `verify` - set true to verify the file against the hash
    ///
    /// # Example
    ///
    /// ```no_run
    /// use manic::downloader;
    /// use manic::Error;
    /// use reqwest::Client;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let client = Client::new();
    ///     let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81";
    ///     let data = downloader::download_and_save(&client, "https://crates.io", 5, Some(hash), "~/Downloads", true).await?;
    ///     Ok(())
    ///  }
    /// ```
    ///
    #[instrument(skip(self))]
    pub async fn download_and_save(&self, path: &str) -> Result<(), Error> {
        let mut result = {
            let name = self.get_filename()?;
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
