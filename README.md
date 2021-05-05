# Manic

[![Crates.io](https://img.shields.io/crates/l/manic)](https://github.com/x0f5c3/manic)
[![Crates.io](https://img.shields.io/crates/v/manic)](https://crates.io/crates/manic)

[![Crates.io](https://img.shields.io/crates/d/manic)](https://crates.io/crates/manic)
[![dependency status](https://deps.rs/crate/manic/0.6.0/status.svg)](https://deps.rs/crate/manic/0.6.0)


Fast and simple async downloads

Provides easy to use functions to download a file using multiple async connections
while taking care to preserve integrity of the file and check it against a SHA256 sum

This crate is a work in progress



### Feature flags

- `progress`: Enables progress reporting using indicatif


### Crate usage

## Examples



```rust
use manic::downloader;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), manic::Error> {
    let client = Client::new();
    let number_of_concurrent_tasks: u8 = 5;
    let result = downloader::download(&client, "https://crates.io", number_of_concurrent_tasks).await?;
    Ok(())
}
```



License: MIT OR Apache-2.0
