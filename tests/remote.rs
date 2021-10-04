use log::LevelFilter;
use manic::{Downloader, Hash};
#[tokio::test]
async fn file_test() -> manic::Result<()> {
    pretty_env_logger::formatted_builder()
        .filter(Some("manic"), LevelFilter::Debug)
        .init();
    let mut dl = Downloader::new(
        "https://github.com/schollz/croc/releases/download/v9.2.0/croc_9.2.0_Windows-64bit.zip",
        3,
    )
    .await?;
    dl.verify(Hash::SHA256(
        "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
    ));
    let _data = dl.download_and_verify().await?;

    Ok(())
}
