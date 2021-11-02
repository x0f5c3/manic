#[cfg(feature = "async")]
mod async_tests;
#[cfg(feature = "threaded")]
mod threaded;

use warp::filters;
use warp::Filter;

pub(crate) async fn start_server(
    port: u16,
    srv_path: Option<&'static str>,
    file_path: Option<&'static str>,
) {
    let inner_srv = srv_path.unwrap_or("croc.zip");
    let inner_file = file_path.unwrap_or("tests/static/croc.zip");
    let file = warp::get()
        .and(filters::path::path(inner_srv))
        .and(filters::path::end())
        .and(filters::fs::file(inner_file));
    warp::serve(file).run(([127, 0, 0, 1], port)).await;
}
