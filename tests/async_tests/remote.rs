use log::LevelFilter;
use manic::{Downloader, Hash};

#[tokio::test]
async fn remote() -> manic::Result<()> {
    let _ = pretty_env_logger::formatted_builder()
        .filter(Some("manic"), LevelFilter::Debug)
        .try_init();
    for i in 1..=5 {
        let mut dl = Downloader::new(
            "https://github.com/schollz/croc/releases/download/v9.2.0/croc_9.2.0_Windows-64bit.zip",
            i,
            None,
        )
        .await?;
        dl.verify(Hash::new_sha256(
            "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
        ));
        let _data = dl.download().await?;
    }
    Ok(())
}
