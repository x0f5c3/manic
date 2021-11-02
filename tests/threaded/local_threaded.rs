use log::LevelFilter;
use manic::{threaded::Downloader, Hash};

#[test]
fn local() -> manic::Result<()> {
    let _ = pretty_env_logger::formatted_builder()
        .filter(Some("manic"), LevelFilter::Debug)
        .filter(Some("warp"), LevelFilter::Info)
        .try_init();
    super::start_threaded(8000, None, None);
    let mut err_vec = Vec::new();
    for i in 1..=10 {
        let mut dl = Downloader::new("http://127.0.0.1:8000/croc.zip", i)?;
        dl.verify(Hash::new_sha256(
            "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
        ));
        let res = dl.download();
        if let Err(e) = res {
            err_vec.push(e);
        }
    }
    if err_vec.is_empty() {
        Ok(())
    } else {
        Err(err_vec.into())
    }
}
