use crate::Error;
use indicatif::ProgressBar;
use reqwest::header::RANGE;
use reqwest::Client;
use tracing::instrument;

#[instrument(skip(client, pb))]
pub async fn download(
    val: String,
    url: &str,
    client: &Client,
    pb: ProgressBar,
) -> Result<Vec<u8>, Error> {
    let mut res = Vec::new();
    let mut resp = client.get(url).header(RANGE, val).send().await?;
    while let Some(chunk) = resp.chunk().await? {
        pb.inc(chunk.len() as u64);
        res.append(&mut chunk.to_vec());
    }
    Ok(res)
}
