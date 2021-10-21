use std::io::Cursor;
use std::sync::Arc;
use futures::{FutureExt, TryStreamExt};
use tokio::sync::{Mutex, MutexGuard};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use crate::chunk::Chunk;
use crate::Result;


pub struct MyMpscCursor {
    cursor: Cursor<Vec<u8>>,
    recv: UnboundedReceiver<Chunk>,
    send: UnboundedSender<Chunk>,
    finish: JoinHandle<Result<()>>,
}


impl MyMpscCursor {
    fn new(inner: T) -> Self {
        let (send, recv) = mpsc::unbounded_channel();
        let curs = Cursor::new(inner);
        let join = tokio::spawn(|| async {
            recv.into_stream().try_for_each_concurrent(|x| async move {
                curs.se
            })
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MyCursor<T: AsRef<[u8]> + Unpin + Clone>(Arc<Mutex<Cursor<T>>>);

impl<T: AsRef<[u8]> + Unpin + Clone> MyCursor<T> {
    pub(crate) fn new(inner: T) -> Self {
        MyCursor(Arc::new(Mutex::new(Cursor::new(inner))))
    }
    pub(crate) async fn as_inner(&self) -> T {
        self.0.lock().await.clone().into_inner()
    }
    pub(crate) async fn lock(&self) -> MutexGuard<'_, Cursor<T>> {
        self.0.lock().await
    }
}
