#![doc(html_root_url = "https://docs.rs/manic_lzfse/0.1.0")]
#![warn(missing_docs)]
/*!
This crate provides an enhanced implementation of the [LZFSE](https://github.com/lzfse/lzfse)
compression library.

### Install

Simply configure your `Cargo.toml`:

```toml
[dependencies]
manic_lzfse = "0.1"
```

### Overview.

This crate provides two LZFSE engines: one operating over user supplied memory buffers, and one operating over internal ring buffers.

The memory buffered engine works directly with input and output buffers that we supply.
It is exposed via [LzfseEncoder] and [LzfseDecoder] objects.
We would consider this engine when operating on `&[u8]` and `Vec<u8>` objects.

The ring buffered engine works by streaming data in and out of it's ring buffers.
It is exposed via [LzfseRingEncoder] and [LzfseRingDecoder] objects.
We would consider this engine when operating on IO streams, or when we want to expose a [Read](std::io::Read) or [Write](std::io::Write) interface.

### Example: compress IO data

This program compresses data from `stdin` into `stdout`. This example can be found in
 `examples/compress_ring.rs`

```no_run
use manic_lzfse::LzfseRingEncoder;
use std::io;

fn main() -> io::Result<()> {
    let mut rdr = io::stdin();
    let mut wtr = io::stdout();
    let mut encoder = LzfseRingEncoder::default();
    encoder.encode(&mut rdr, &mut wtr)?;
    Ok(())
}

```

### Example: decompress IO data

This program decompresses data from `stdin` into `stdout`. This example can be found in
 `examples/decompress_ring.rs`

```no_run
use manic_lzfse::LzfseRingDecoder;
use std::io;

fn main() -> io::Result<()> {
    let mut rdr = io::stdin();
    let mut wtr = io::stdout();
    let mut decoder = LzfseRingDecoder::default();
    decoder.decode(&mut rdr, &mut wtr)?;
    Ok(())
}

```

### Example: compress buffered data

This program compresses data from `stdin` into `stdout`. This example can be found in
 `examples/compress.rs`

```no_run
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    // Read stdin into src.
    let mut rdr = io::stdin();
    let mut src = Vec::default();
    rdr.read_to_end(&mut src)?;

    // Compress src into dst.
    let mut dst = Vec::default();
    manic_lzfse::encode_bytes(&src, &mut dst)?;

    // Write dst into stdout.
    let mut wtr = io::stdout();
    wtr.write_all(&dst)?;

    Ok(())
}
```

### Example: decompress buffered data

This program decompresses data from `stdin` into `stdout`. This example can be found in
 `examples/decompress.rs`

```no_run
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    // Read stdin into src.
    let mut rdr = io::stdin();
    let mut src = Vec::default();
    rdr.read_to_end(&mut src)?;

    // Compress src into dst.
    let mut dst = Vec::default();
    manic_lzfse::encode_bytes(&src, &mut dst)?;

    // Write dst into stdout.
    let mut wtr = io::stdout();
    wtr.write_all(&dst)?;

    Ok(())
}
```
*/

mod base;
mod bits;
mod decode;
mod encode;
mod error;
mod fse;
mod kit;
mod lmd;
mod lz;
mod match_kit;
mod ops;
mod raw;
mod ring;
mod types;
mod vn;

#[cfg(test)]
pub mod test_utils;

pub use decode::{decode_bytes, LzfseDecoder, LzfseReader, LzfseReaderBytes, LzfseRingDecoder};
pub use encode::{encode_bytes, LzfseEncoder, LzfseRingEncoder, LzfseWriter, LzfseWriterBytes};
pub use error::{Error, Result};
pub use fse::FseErrorKind;
pub use vn::VnErrorKind;

#[cfg(test)]
mod tests {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("README.md");
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!("src/lib.rs");
    }
}
