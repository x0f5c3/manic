# par_download

![Workflow Status](https://github.com/x0f5c3/par_download/workflow/Test-and-Clippy/badge.svg)
[![codecov](https://codecov.io/gh/x0f5c3/par_download/branch/master/graph/badge.svg)](https://codecov.io/gh/x0f5c3/par_download)

Fast and simple async downloads

Provides easy to use functions to download a file using multiple async connections
while taking care to preserve integrity of the file and check it against a SHA256 sum

This crate is a work in progress



### Feature flags

- `progress`: Enables progress reporting using indicatif


### Crate usage

## Examples



```rust
use par_download::downloader;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), par_download::Error> {
    let client = Client::new();
    let number_of_concurrent_tasks: u8 = 5;
    let result = downloader::download(&client, "https://crates.io", number_of_concurrent_tasks).await?;
    Ok(())
}
```



License: MIT OR Apache-2.0
