use download::download;
use par_download::download;
use indicatif::{ProgressBar, ProgressStyle};



#[tokio::main]
async fn main() {
    let url = "http://127.0.0.1:8080/kraken.deb";
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("#>-");
    let pb = ProgressBar::new(100);
    pb.set_style(style);
    let client = reqwest::Client::new();
    let fake_path = "test".to_string();
    let res = download::download_with_progress(&client, url, 5 as usize, pb).await.unwrap();
    download::compare_sha(res.as_slice(), "9ba07a4b089767fe3bf553a2788b97ea1909a724f67d8410b18048e845eec3e8".to_string()).unwrap();
}

