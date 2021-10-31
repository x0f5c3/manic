use log::LevelFilter;
use manic::threaded::{Downloader, Hash};
#[test]
fn file_test() -> manic::threaded::Result<()> {
    pretty_env_logger::formatted_builder()
        .filter(Some("manic"), LevelFilter::Debug)
        .init();
    let mut dl = Downloader::new(
        "https://github.com/schollz/croc/releases/download/v9.2.0/croc_9.2.0_Windows-64bit.zip",
        3,
    )?;
    dl.verify(Hash::new_sha256(
        "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
    ));
    let _data = dl.download()?;

    Ok(())
}
