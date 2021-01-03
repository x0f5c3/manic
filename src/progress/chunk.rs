use crate::{Connector, Error};
use hyper::header::RANGE;
use hyper::Client;
use tokio_stream::StreamExt;
use indicatif::ProgressBar;
use tracing::instrument;

#[instrument(skip(client, pb))]
pub async fn download_chunk(
    val: String,
    url: &str,
    client: &Client<impl Connector>,
    pb: &ProgressBar,
) -> Result<Vec<u8>, Error> {
    let mut res = Vec::new();
    let req = hyper::Request::get(url)
        .header(RANGE, val)
        .body(hyper::Body::empty())?;
    let mut resp = client.request(req.into()).await?.into_body();
    while let Some(Ok(chunk)) = resp.next().await {
        pb.inc(chunk.len() as u64);
        res.append(&mut chunk.to_vec());
    }
    Ok(res)
}
