#![allow(dead_code)]
use super::chunk::ChunkVec;
use super::downloader::join_all;
use super::error::ManicError;
use super::Result;
use super::{Downloader, Hash};
use indicatif::{MultiProgress, ProgressBar};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};
use std::thread;

#[derive(Clone, Debug)]
pub struct Map(Arc<Mutex<HashMap<String, Downloader>>>);

impl Map {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, HashMap<String, Downloader>>> {
        self.0
            .lock()
            .map_err(|e| ManicError::MultipleErrors(e.to_string()))
    }
    pub(crate) fn as_inner(&self) -> &Arc<Mutex<HashMap<String, Downloader>>> {
        &self.0
    }
    pub(crate) fn into_inner(self) -> Arc<Mutex<HashMap<String, Downloader>>> {
        self.0
    }
    pub(crate) fn insert(&self, k: String, v: Downloader) -> Result<Option<Downloader>> {
        let mut lock = self.lock()?;
        Ok(lock.insert(k, v))
    }
    pub(crate) fn get(&self, k: &str) -> Result<Downloader> {
        let lock = self.lock()?;
        let res = lock.get(k);
        res.cloned().ok_or(ManicError::NotFound)
    }
}

#[derive(Debug, Clone)]
pub struct Downloaded {
    url: String,
    name: String,
    data: ChunkVec,
}

impl Downloaded {
    pub(crate) fn new(url: String, name: String, data: ChunkVec) -> Self {
        Self { url, name, data }
    }
    pub(crate) fn save<T: AsRef<Path>>(&self, output_dir: T) -> Result<()> {
        let output_path = output_dir.as_ref().join(Path::new(&self.name));
        self.data.save_to_file(output_path)
    }
}

#[derive(Debug)]
pub struct MultiDownloader {
    downloaders: Map,
    #[cfg(feature = "progress")]
    progress: Option<MultiProgress>,
}

impl MultiDownloader {
    pub fn new(progress: bool) -> MultiDownloader {
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
    pub fn add(&mut self, url: String, workers: u8) -> Result<()> {
        let mut client = Downloader::new(&url, workers)?;
        #[cfg(feature = "progress")]
        if let Some(pb) = &self.progress {
            let mpb = ProgressBar::new(client.get_len());
            let to_add = pb.add(mpb);
            client.connect_progress(to_add);
        }
        self.downloaders.insert(url, client)?;
        Ok(())
    }
    pub fn verify(&mut self, url: String, hash: Hash) -> Result<()> {
        let mut lock = self.downloaders.lock()?;
        let chosen: &mut Downloader = lock.get_mut(&url).ok_or(ManicError::NotFound)?;
        let modified = chosen.verify(hash);
        lock.insert(url, modified).ok_or(ManicError::NotFound)?;
        Ok(())
    }
    pub fn download_all(&self) -> Result<Vec<Downloaded>> {
        let mut fut_vec = Vec::new();
        let lock = self.downloaders.lock()?;
        for v in lock.values() {
            let c = v.clone();
            fut_vec.push(thread::spawn(|| c.multi_download()));
        }
        Ok(join_all(fut_vec)?.to_vec())
    }
    pub fn download_one(&self, url: String) -> Result<ChunkVec> {
        let chosen = self.downloaders.get(&url)?;
        chosen.download()
    }
}
