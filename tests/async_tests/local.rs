use log::LevelFilter;
use manic::{Downloader, Hash, ManicError, Result};
use std::time::Duration;

#[tokio::test]
async fn local() -> Result<()> {
    let _ = pretty_env_logger::formatted_builder()
        .filter(Some("manic"), LevelFilter::Debug)
        .filter(Some("warp"), LevelFilter::Info)
        .try_init();
    tokio::spawn(crate::start_server(8001, None, None));
    tokio::time::sleep(Duration::from_secs(3)).await;
    let mut res_vec: Vec<ManicError> = Vec::new();
    let mut is_err = false;
    for i in 1..=10 {
        let mut dl = Downloader::new("http://127.0.0.1:8001/croc.zip", i, None).await?;
        dl.verify(Hash::new_sha256(
            "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
        ));
        if let Err(e) = dl.download().await {
            is_err = true;
            res_vec.push(e);
        }
    }
    if is_err {
        return Err(res_vec.into());
    }
    Ok(())
}
