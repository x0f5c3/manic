use crate::{ManicError, Result};
use futures::Future;
use tokio::task::JoinHandle;

pub(crate) async fn join_all<T: Clone>(i: Vec<JoinHandle<Result<T>>>) -> Result<Vec<T>> {
    let results = futures::future::join_all(i)
        .await
        .into_iter()
        .filter_map(|x| x.ok())
        .collect::<Vec<_>>();
    let errs = results
        .iter()
        .filter_map(|x| x.as_ref().err())
        .cloned()
        .collect::<Vec<_>>();
    let successful = results
        .iter()
        .filter_map(|x| x.as_ref().ok())
        .cloned()
        .collect::<Vec<T>>();
    check_err(errs, successful)
}
pub(crate) async fn join_all_futures<T: Clone, F: Future<Output = Result<T>>>(
    i: Vec<F>,
) -> Result<Vec<T>> {
    let res = futures::future::join_all(i).await;
    let errs = res
        .iter()
        .filter_map(|x| x.as_ref().err())
        .cloned()
        .collect::<Vec<ManicError>>();
    let successful = res.into_iter().filter_map(|x| x.ok()).collect::<Vec<T>>();
    check_err(errs, successful)
}

pub(crate) fn check_err<T: Clone>(err: Vec<ManicError>, good: Vec<T>) -> Result<Vec<T>> {
    if !err.is_empty() && good.is_empty() {
        Err(err.into())
    } else if !good.is_empty() {
        Ok(good)
    } else {
        Err(ManicError::NoResults)
    }
}
