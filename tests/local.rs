
#[cfg(any(all(feature = "rustls-tls", not(feature = "openssl-tls")), all(feature = "openssl-tls", not(feature = "rustls-tls"))))]
use manic::Downloader;

#[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
type Downloader = manic::Downloader<manic::Rustls>;


#[tokio::test]
async fn file_test() -> manic::Result<()> {

    let mut dl = Downloader::new("https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-10.7.0-amd64-netinst.iso", 3).await?;
    dl.verify(manic::Hash::SHA256("b317d87b0a3d5b568f48a92dcabfc4bc51fe58d9f67ca13b013f1b8329d1306d".to_string()));
    let _data = dl.download_and_verify().await?;

    Ok(())
}