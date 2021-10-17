use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

#[derive(Clone, Debug)]
pub struct MyCursor<T: AsRef<[u8]> + Unpin + Clone>(Arc<Mutex<Cursor<T>>>);

impl<T: AsRef<[u8]> + Unpin + Clone> MyCursor<T> {
    pub fn new(inner: T) -> Self {
        MyCursor(Arc::new(Mutex::new(Cursor::new(inner))))
    }
    pub async fn as_inner(&self) -> T {
        self.0.lock().await.clone().into_inner()
    }
    pub async fn lock(&self) -> MutexGuard<'_, Cursor<T>> {
        self.0.lock().await
    }
}
