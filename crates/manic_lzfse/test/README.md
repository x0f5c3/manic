# test

API tests.

## Basic usage

Quick test:
```
$ cargo test --manifest-path test/Cargo.toml 
```

Full test:
```
$ cargo test --manifest-path test/Cargo.toml -- --ignored
```

Extended test:
```
$ RUSTFLAGS="-L /usr/local/lib/x86_64-linux-gnu" cargo test --manifest-path test/Cargo.toml --features "large_data huge_data lzfse_ref" -- --ignored
```

The quick test takes minutes. The full test takes hours. The extended test takes many hours.


## Cross compilation

Install and setup the [cross](https://github.com/rust-embedded/cross) crate.
We can then test against alternative architectures.

Mips: 32 bit, big endian
```
$ cross test --target mips-unknown-linux-gnu --manifest-path test/Cargo.toml
```

Arm: 32 bit, little endian
```
$ cross test --target armv7-unknown-linux-gnueabihf --manifest-path test/Cargo.toml
```

Arm: 64 bit, little endian
```
$ cross test --target aarch64-unknown-linux-gnu --manifest-path test/Cargo.toml
```

## Large data

Test large data files.

Before enabling we need to download, hash and compress the large data file set into `data/large`. As a prerequisite we require the reference [LZFSE](https://github.com/lzfse/lzfse) binary and a working internet connection.

Then from the project root:
```
$ ./scripts/init_large.sh

```

We can then pass the `large_data` feature flag to enable large data tests.

```
$ cargo test large --manifest-path test/Cargo.toml --features large_data
```
```
$ cargo test --manifest-path test/Cargo.toml --features large_data
```


## Lzfse reference

Test `manic_lzfse`/ `lzfse` compatibility. Here `manic_lzfse` compressed data is handed over to `lzfse` to decompress and vice versa.

For this to work, we need to build the reference LZFSE `liblzfse.a` library and inform `rustc` of it's whereabouts. See the `lzfse_sys/README.md` for instructions on how to do this.

```
$ RUSTFLAGS="-L /usr/local/lib/x86_64-linux-gnu" cargo test --manifest-path test/Cargo.toml --features lzfse_ref -- --ignored
```


## Huge data

Test huge virtual synthetic data files using concurrent `manic_lzfse` process invocations.
Although we are testing 64GB+ data files the actual memory requirements should not exceed 2MB.

```
$ cargo test huge --manifest-path test/Cargo.toml --features huge_data
```

```
$ cargo test --manifest-path test/Cargo.toml --features huge_data
```


## Test organization

The library exposes:
* encoders: `LzfseEncoder`, `LzfseRingEncoder`
* encode writers: `LzfseWriterBytes`, `LzfseWriter`
* decoders: `LzfseDecoder`, `LzfseRingDecoder`
* decode readers: `LzfseReaderBytes`, `LzfseReader`

We need to ensure that the compression and decompression methods work as intended and that data corruption does not occur.
Additionally the library is designed to validate and reject input data; it should not hang, segfault, panic or break in a any other fashion.
Internally the code base is packed with debug statements that trip on invalid states, these are hard errors and should NOT occur.

Quick tests.
Small data sets and fast execution patterns.

* data - [`Snappy`](https://google.github.io/snappy/) data set.
* pattern - synthetic data pattern variations.

Extended tests.
We resort to throwing huge amounts of valid and invalid data at the API.

* large data - large data files: 100MB+.
* huge data - huge virtual synthetic data files: 64GB+.
* pattern - synthetic data pattern variations.
* patchwork - patchwork data.
* mutate - RAW, Vn, Vx1, Vx2 data mutation.
* fuzz - fuzzed read/ write.
* random - low entropy random data.