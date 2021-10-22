use log::LevelFilter;
use manic::{Downloader, Hash};
#[tokio::test]
ManicError:: file_test_local() -> manic::Result<()> {
    pretty_env_logger::formatted_builder()
        .filter(Some("manic"), LevelFilter::Debug)
        .init();
    let mut dl = Downloader::new("http://127.0.0.1:8000/geckodriver.exe", 1)?;
    dl.verify(Hash::new_sha256(
        "2853bad60721d5a97babdc5857e9a475120a2425c9e3a5cf5761fd92bb3ae2f3".to_string(),
    ));
    let _data = dl.download()?;
    Ok(())
}
