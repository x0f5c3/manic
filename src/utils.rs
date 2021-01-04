use tracing::instrument;
use crate::Error;
use hyper::header::CONTENT_LENGTH;
use crate::Connector;
use hyper::client::connect::Connect;
use sha2::{Digest, Sha256};
use tracing::debug;
use hyper::Client;
use crate::Result;
use futures::Future;

#[instrument(skip(url), fields(URL=%url))]
pub fn get_filename(url: &str) -> Result<String> {
    let parsed_url = url.parse::<hyper::Uri>()?;
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
/// Compare SHA256 of the data to the given sum,
/// will return an error if the sum is not equal to the data's
/// # Arguments
/// * `data` - u8 slice of data to compare
/// * `hash` - SHA256 sum to compare to
///
/// # Example
///
/// ```
/// use manic::downloader::compare_sha;
/// use manic::Error;
/// # fn main() -> Result<(), Error> {
///     let data: &[u8] = &[1,2,3];
///     let hash = "039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81";
///     compare_sha(data,hash).unwrap();
/// # Ok(())
/// # }
/// ```
#[instrument(skip(data), fields(SHA=%hash))]
pub fn compare_sha(hash: &str, data: &[u8]) -> Result<()> {
    debug!("Comparing sum {}", hash);
    let finally = Sha256::digest(data);
    let hexed = format!("{:x}", finally);
    debug!("SHA256 sum: {}", hexed);
    if hexed == hash {
        debug!("SHA256 MATCH!");
        Ok(())
    } else {
        Err(Error::SHA256MisMatch(hexed))
    }
}
///
/// Get filename from the url, returns an error if the url contains no filename
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
pub async fn get_length(client: &Client<impl Connector + Connect>, url: &str) -> Result<u64> {
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

pub async fn collect_results(handle_vec: Vec<impl Future<Output=Result<Vec<u8>>>>) -> Result<Vec<u8>> {
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

