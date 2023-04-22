mod app;
mod client;

// use crate::app::App;
use manic_http::threaded::{Downloader, MultiDownloader};

fn main() {
    // let app: App = App::new();
    // app.init_logging();
    // let threads = if let Some(t) = app.threads { t } else { 4 };
    // match app.urls.len() {
    //     d if d >= 2 => {
    //         let mut dl: MultiDownloader = MultiDownloader::new(true, threads);
    //         for i in app.urls {
    //             dl.add(i)?;
    //         }
    //         let res = dl.download_all()?;
    //         for i in res {
    //             if let Some(p) = &app.output {
    //                 i.save(&p, dl.get_pool())?;
    //             } else {
    //                 i.save("..", dl.get_pool())?;
    //             }
    //         }
    //     }
    //     _ => {
    //         let mut dl = Downloader::new(&app.urls[0], threads)?;
    //         dl.progress_bar();
    //         if let Some(p) = &app.output {
    //             dl.download_and_save(p.to_str().context("Failed to convert path to string")?)?;
    //         } else {
    //             dl.download_and_save(".")?;
    //         }
    //     }
    // }
}
