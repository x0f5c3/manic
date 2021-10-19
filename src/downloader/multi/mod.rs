#![allow(dead_code)]
use crate::chunk::ChunkVec;
use crate::error::ManicError;
use crate::Result;
use crate::{join_all, join_all_futures};
use crate::{Downloader, Hash};
use indicatif::{MultiProgress, ProgressBar};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, MutexGuard};

#[derive(Clone, Debug)]
pub struct Map(Arc<Mutex<HashMap<String, Downloader>>>);

impl Map {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
    pub(crate) async fn lock(&self) -> MutexGuard<'_, HashMap<String, Downloader>> {
        self.0.lock().await
    }
    pub(crate) fn as_inner(&self) -> &Arc<Mutex<HashMap<String, Downloader>>> {
        &self.0
    }
    pub(crate) fn into_inner(self) -> Arc<Mutex<HashMap<String, Downloader>>> {
        self.0
    }
    pub(crate) async fn insert(&self, k: String, v: Downloader) -> Option<Downloader> {
        let mut lock = self.lock().await;
        lock.insert(k, v)
    }
    pub(crate) async fn get(&self, k: &str) -> Result<Downloader> {
        let lock = self.lock().await;
        let res = lock.get(k);
        res.cloned().ok_or(ManicError::NotFound)
    }
}

#[derive(Debug, Clone)]
pub struct Downloaded {
    url: String,
    name: String,
    data: Vec<u8>,
}

impl Downloaded {
    pub(crate) fn new(url: String, name: String, data: Vec<u8>) -> Self {
        Self { url, name, data }
    }
    pub(crate) async fn save<T: AsRef<Path>>(&self, output_dir: T) -> Result<()> {
        let output_path = output_dir.as_ref().join(Path::new(&self.name));
        let mut f = File::create(output_path).await?;
        f.write_all(self.data.as_slice())
            .await
            .map_err(|x| x.into())
    }
}

#[derive(Debug)]
pub struct MultiDownloader {
    downloaders: Map,
    #[cfg(feature = "progress")]
    progress: Option<MultiProgress>,
}

impl MultiDownloader {
    pub async fn new(progress: bool) -> MultiDownloader {
        #[cfg(feature = "progress")]
        let pb = if progress {
            Some(MultiProgress::new())
        } else {
            None
        };
        Self {
            downloaders: Map::new(),
            #[cfg(feature = "progress")]
            progress: pb,
        }
    }
    pub async fn add(&mut self, url: String, workers: u8) -> Result<()> {
        let mut client = Downloader::new(&url, workers).await?;
        #[cfg(feature = "progress")]
        if let Some(pb) = &self.progress {
            let mpb = ProgressBar::new(client.get_len());
            let to_add = pb.add(mpb);
            client.connect_progress(to_add);
        }
        self.downloaders.insert(url, client).await;
        Ok(())
    }
    pub async fn verify(&mut self, url: String, hash: Hash) -> Result<()> {
        let mut lock = self.downloaders.lock().await;
        let chosen = lock.get_mut(&url).ok_or(ManicError::NotFound)?;
        chosen.verify(hash);
        Ok(())
    }
    pub async fn download_all(&self) -> Result<Vec<Downloaded>> {
        let mut fut_vec = Vec::new();
        let lock = self.downloaders.lock().await;
        for v in lock.values() {
            let c = v.clone();
            fut_vec.push(tokio::spawn(c.multi_download()));
        }
        Ok(join_all(fut_vec).await?.to_vec())
    }
    pub async fn download_one(&self, url: String) -> Result<ChunkVec> {
        let chosen = self.downloaders.get(&url).await?;
        chosen.download().await
    }
}
