use manic::{Downloader, Hash};

#[tokio::test]
async fn file_test() -> manic::Result<()> {
    pretty_env_logger::init();
    let mut dl = Downloader::new(
        "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-10.7.0-amd64-netinst.iso",
        3,
    )
    .await?;
    dl.verify(Hash::SHA256(
        "b317d87b0a3d5b568f48a92dcabfc4bc51fe58d9f67ca13b013f1b8329d1306d".to_string(),
    ));
    let _data = dl.download_and_verify().await?;

    Ok(())
}
