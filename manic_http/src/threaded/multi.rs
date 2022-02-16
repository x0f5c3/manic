#![allow(dead_code)]

use super::chunk::ChunkVec;
use super::downloader::join_all;
use super::Downloader;
use crate::{Hash, ManicError, Result};
#[cfg(feature = "progress")]
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rusty_pool::ThreadPool;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};

#[derive(Clone)]
pub struct Map(Arc<Mutex<HashMap<String, Downloader>>>);

impl Default for Map {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

impl Map {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, HashMap<String, Downloader>>> {
        self.0
            .lock()
            .map_err(|e| ManicError::PoisonError(e.to_string()))
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
    pub fn save<T: AsRef<Path>>(&self, output_dir: T, pool: ThreadPool) -> Result<()> {
        let output_path = output_dir.as_ref().join(Path::new(&self.name));
        self.data.save_to_file(output_path, pool)
    }
}

#[derive(Clone, Builder)]
pub struct MultiDownloader {
    #[builder(default)]
    downloaders: Map,
    #[builder(default, setter(skip))]
    #[cfg(feature = "progress")]
    progress: Option<Arc<MultiProgress>>,
    #[cfg(feature = "progress")]
    progress_style: Option<ProgressStyle>,
    #[builder(default, setter(skip))]
    pool: ThreadPool,
    workers: u8,
}

impl MultiDownloader {
    pub fn new(#[cfg(feature = "progress")] progress: bool, workers: u8) -> MultiDownloader {
        #[cfg(feature = "progress")]
        let pb = if progress {
            Some(Arc::new(MultiProgress::new()))
        } else {
            None
        };
        let pool = rusty_pool::Builder::new()
            .max_size(workers as usize)
            .build();
        Self {
            downloaders: Map::new(),
            #[cfg(feature = "progress")]
            progress: pb,
            #[cfg(feature = "progress")]
            progress_style: None,
            pool,
            workers,
        }
    }
    pub fn add(&mut self, url: String) -> Result<()> {
        #[allow(unused_mut)]
        let mut client = Downloader::new_multi(&url, self.workers, self.pool.clone())?;
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
            fut_vec.push(self.pool.evaluate(|| c.multi_download()));
        }
        Ok(join_all(fut_vec)?.to_vec())
    }
    pub fn download_one(&self, url: String) -> Result<ChunkVec> {
        let chosen = self.downloaders.get(&url)?;
        chosen.download()
    }
    pub fn get_pool(&self) -> ThreadPool {
        self.pool.clone()
    }
}
