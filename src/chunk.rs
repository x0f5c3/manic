use reqwest::Client;
use thiserror::Error;
use reqwest::header::RANGE;
#[cfg(feature = "progress")]
use indicatif::ProgressBar;




pub struct Chunk<'a> {
    url: &'a str,
    range: Range,
}

pub enum Range {
    Last(u64),
    Normal((u64,u64)),
}



impl<'a> Chunk<'a> {
    pub fn new(url: &'a str, range: Range) -> Self {
        Chunk {
            url,
            range,
        }
    }
    pub async fn download(self, client: &Client) -> Result<Vec<u8>, Error> {
        let val = match self.range {
            Range::Last(index) => format!("bytes={}-", index),
            Range::Normal((first, last)) => format!("bytes={}-{}", first, last),
        };
        let resp = client.get(self.url).header(RANGE, val).send().await?.bytes().await?;
        Ok(resp.as_ref().to_vec())
    }

    #[cfg(feature = "progress")]
    pub async fn download_with_progress(self, client: &Client, pb: ProgressBar) -> Result<Vec<u8>, Error> {
        let val = match self.range {
            Range::Last(index) => format!("bytes={}-", index),
            Range::Normal((first, last)) => format!("bytes={}-{}", first, last),
        };
        let mut res = Vec::new();
        let mut resp = client.get(self.url).header(RANGE, val).send().await?;
        while let Some(chunk) = resp.chunk().await? {
            pb.inc(chunk.len() as u64);
            res.append(&mut chunk.to_vec());
        }
        Ok(res)
        
    }


    
}






#[derive(Debug, Error)]
pub enum Error {
    #[error("Network error: {0}")]
    NetError(#[from] reqwest::Error),
}






