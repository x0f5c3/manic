use crate::Connector;
use crate::Result;
use crate::{Error, Hash};
use futures::Future;
use hyper::client::connect::Connect;
use hyper::header::CONTENT_LENGTH;
use hyper::Client;
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};
use tracing::debug;
use tracing::instrument;
use http::StatusCode;
use http::header::LOCATION;

/// Get filename from the url, returns an error if the url contains no filename
#[instrument(skip(url), fields(URL=%url))]
pub(crate) fn get_filename(url: &str) -> Result<String> {
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
/// Get the content-length header using a head request
///
/// # Arguments
///
/// * `client` - reference to a hyper [`Client`][hyper::Client] with Https type, [`Rustls`] or [`OpenSSL`]
/// * `url` - &str with the url
///
/// # Example
///
///```no_run
/// #[cfg(feature = "rustls-tls")]
/// use manic::Rustls as Conn;
/// #[cfg(feature = "openssl-tls")]
/// use manic::OpenSSL as Conn;
/// use manic::Connector;
/// use manic::Error;
/// use hyper::Client;
/// use manic::utils::get_length;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
///     let connector = Conn::new();
///     let client = Client::builder().build::<_, hyper::Body>(connector);
///     let length = get_length(&client, "https://docs.rs").await.unwrap();
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

pub async fn check_redirects(client: &Client<impl Connector + Connect>, url: &str) -> Result<Option<String>> {
    let req = hyper::Request::head(url)
        .body(hyper::Body::empty())
        .map_err(|e| Error::REQError(e))?;
    let head_req = client.request(req.into()).await?;
    let status = head_req.status().as_u16();
    if status == 301 || status == 308 || status == 302 || status == 303 || status == 307 {
        let loc = head_req.headers()[LOCATION].to_str()?;
        let uri = loc.parse::<hyper::Uri>()?;
        let path = if uri.host().is_some() {
            loc.to_string()
        } else {
            let original = url.parse::<hyper::Uri>()?;
            let mut part = original.into_parts();
            let new_path = uri.path_and_query().ok_or(Error::BadChunkSize)?.to_owned();
            part.path_and_query = Some(new_path);
            let new_url = hyper::Uri::from_parts(part).unwrap().to_string();
            new_url

        };
        Ok(Some(path))
    } else {
        Ok(None)
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
