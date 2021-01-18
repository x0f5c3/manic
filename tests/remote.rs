use manic::{Downloader, Hash};

#[tokio::test]
async fn file_test() -> manic::Result<()> {
    pretty_env_logger::init();
    let mut dl = Downloader::new(
        "https://dl-cdn.alpinelinux.org/alpine/v3.13/releases/x86_64/alpine-minirootfs-3.13.0-x86_64.tar.gz",
        3,
    )
    .await?;
    dl.verify(Hash::SHA256(
        "37b7dc2877bdfbe399e076db02facc81862fb3ee130c37eaa14a35f547eeb1d3".to_string(),
    ));
    let _data = dl.download_and_verify().await?;

    Ok(())
}
