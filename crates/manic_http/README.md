# Manic

[![Crates.io](https://img.shields.io/crates/l/manic)](https://github.com/x0f5c3/manic)
[![Crates.io](https://img.shields.io/crates/v/manic)](https://crates.io/crates/manic)
![Tests](https://github.com/x0f5c3/manic/actions/workflows/fmt_and_clippy.yml/badge.svg)

[![Crates.io](https://img.shields.io/crates/d/manic)](https://crates.io/crates/manic)
[![dependency status](https://deps.rs/crate/manic/0.6.4/status.svg)](https://deps.rs/crate/manic/0.6.4)


Fast and simple multithread downloads

Provides easy to use functions to download a file using multiple async or threaded connections
while taking care to preserve integrity of the file and check it against a checksum.


## Feature flags

- `progress`: Enables progress reporting using indicatif [enabled by default] 
- `json`: Enables use of JSON features on the reqwest Client [enabled by default]
- `rustls`: Use Rustls for HTTPS [enabled by default]
- `openssl`: Use OpenSSL for HTTPS
- `threaded`: Enable multithreaded client
- `async`: Enable async client [enabled by default]


## Crate usage

### Examples

#### Async example

```rust
use manic_http::Downloader;

#[tokio::main]
async fn main() -> Result<(), manic::ManicError> {
	let workers: u8 = 5;
	let client = Downloader::new("https://crates.io", workers).await?;
	let _ = client.download().await?;
	Ok(())
}
```

#### Multithread example

```rust
use manic_http::threaded::Downloader;

fn main() -> Result<(), manic::ManicError> {
    let workers: u8 = 5;
    let client = Downloader::new("https://crates.io", workers)?;
    let _ = client.download()?;
    Ok(())
}
```



License: MIT OR Apache-2.0
