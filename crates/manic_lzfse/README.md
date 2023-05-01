# manic_lzfse
Rust LZFSE implementation.


## Documentation

https://docs.rs/manic_lzfse

## Install

Simply configure your `Cargo.toml`:

```toml
[dependencies]
manic_lzfse = "0.1"
```


## Overview

This crate provides two LZFSE engines: one operating over user supplied memory buffers, and one operating over internal ring buffers.

The memory buffered engine works directly with input and output buffers that we supply.
It is exposed via `LzfseEncoder` and `LzfseDecoder` objects.
We would consider this engine when operating on `&[u8]` and `Vec<u8>` objects.

The ring buffered engine works by streaming data in and out of it's ring buffers.
It is exposed via `LzfseRingEncoder` and `LzfseRingDecoder` objects.
We would consider this engine when operating on IO streams, or when we want to expose a `Read` or `Write` interface.

Check the documentation for additional information and examples.


## Examples

This program compresses data from `stdin` into `stdout`. This example can be found in
 `examples/compress_ring.rs`

```rust
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

This program decompresses data from `stdin` into `stdout`. This example can be found in
 `examples/decompress_ring.rs`

```rust
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


# Command line tool: lzfoo

A fast, memory efficient and stream capable [lzfse](https://github.com/lzfse/lzfse) command line tool clone.
Powered by [manic_lzfse](https://github.com/shampoofactory/manic_lzfse).

Install.

```
$ cargo install lzfoo
```

Compress `a.txt` to `a.txt.lzfse`:
```
$ lzfoo -encode -i a.txt -o a.txt.lzfse
```

Compress with stdin/ stdout:
```
$ lzfoo -encode -i < a.txt > a.txt.lzfse
```
```
$ echo "semper fidelis" | lzfoo -encode > a.txt.lzfse
```

Decompress `a.txt.lzfse` to `a.txt`:
```
$ lzfoo -decode -i a.txt.lzfse -o a.txt
```

Decompress with stdin/ stdout:
```
$ lzfoo -decode -i < a.txt.lzfse > a.txt
```

Check the [lzfoo crate](https://github.com/shampoofactory/manic_lzfse/tree/main/lzfoo) for details.


## Testing

This crate comes with a comprehensive test suite that is divided into unit tests and integration tests.
The unit tests check that the library's internal components are working, whilst the integration tests check that library as a whole is working.
Extended tests are available but may take hours to complete.

Unit tests:

```
$ cargo test
```

Unit tests, extended:

```
$ cargo test -- --ignored
```

Integration tests:

```
$ cargo test --manifest-path test/Cargo.toml
```

Integration tests, extended:

```
$ cargo test --manifest-path test/Cargo.toml -- --ignored
```

Additional integration tests are available.
Notably, validation tests with the reference LZFSE implementation.
These are described in [test](https://github.com/shampoofactory/manic_lzfse/tree/main/test), along with instructions on how to build and run them.


## Performance

As a library and with the stated test machine, `manic_lzfse` outperforms `lzfse_ref`. However results will vary with different machines.

The benchmarks are powered with [Criterion](https://github.com/bheisler/criterion.rs) and formatted with [critcmp](https://github.com/BurntSushi/critcmp).
The machine is an 8GB Intel i5-2500K running [Ubuntu](https://ubuntu.com/) 18.04 (64 bit).
The dataset is taken from [Snappy](https://github.com/google/snappy).
The benchmark source is [here](https://github.com/shampoofactory/manic_lzfse/tree/main/bench), along with instructions on how to build and run the benchmarks.

```
group                          new/lzfse_ref/                         new/rust/                              new/rust_ring/
-----                          --------------                         ---------                              --------------
decode/snap_uflat00_html       1.24    119.6±0.11µs   816.8 MB/sec    1.07    103.3±0.03µs   945.7 MB/sec    1.00     96.2±0.04µs  1014.9 MB/sec
decode/snap_uflat01_urls       1.18   1407.8±3.84µs   475.6 MB/sec    1.02   1211.8±0.11µs   552.5 MB/sec    1.00   1193.0±3.27µs   561.2 MB/sec
decode/snap_uflat02_jpg        1.07    353.7±0.05µs   331.9 MB/sec    1.00    330.7±0.91µs   355.0 MB/sec    1.02    336.1±0.85µs   349.3 MB/sec
decode/snap_uflat04_pdf        1.07    243.6±0.07µs   400.8 MB/sec    1.00    227.7±0.04µs   429.0 MB/sec    1.00    228.7±0.04µs   427.0 MB/sec
decode/snap_uflat05_html4      1.13    140.1±0.08µs     2.7 GB/sec    1.00    123.4±0.36µs     3.1 GB/sec    1.12    138.4±0.04µs     2.8 GB/sec
decode/snap_uflat06_txt1       1.22    469.4±0.05µs   309.0 MB/sec    1.09    420.8±1.21µs   344.7 MB/sec    1.00    384.3±0.05µs   377.4 MB/sec
decode/snap_uflat07_txt2       1.20    410.2±1.18µs   291.0 MB/sec    1.10    373.4±0.01µs   319.7 MB/sec    1.00    340.9±0.02µs   350.2 MB/sec
decode/snap_uflat08_txt3       1.25   1255.5±0.12µs   324.2 MB/sec    1.09   1096.9±3.32µs   371.0 MB/sec    1.00   1006.9±0.15µs   404.2 MB/sec
decode/snap_uflat09_txt4       1.18   1628.8±0.25µs   282.1 MB/sec    1.10   1511.5±3.03µs   304.0 MB/sec    1.00   1376.4±0.11µs   333.9 MB/sec
decode/snap_uflat10_pb         1.17    101.7±0.04µs  1112.3 MB/sec    1.04     90.2±0.03µs  1254.1 MB/sec    1.00     86.7±0.04µs  1304.3 MB/sec
decode/snap_uflat11_gaviota    1.28    486.0±0.05µs   361.7 MB/sec    1.09    413.2±0.05µs   425.4 MB/sec    1.00    379.5±0.03µs   463.1 MB/sec
encode/snap_uflat00_html       1.83   1500.0±3.76µs    65.1 MB/sec    1.00    821.1±0.09µs   118.9 MB/sec    1.10    905.9±0.11µs   107.8 MB/sec
encode/snap_uflat01_urls       1.45     13.1±0.00ms    51.3 MB/sec    1.00      9.0±0.00ms    74.2 MB/sec    1.01      9.1±0.00ms    73.8 MB/sec
encode/snap_uflat02_jpg        1.11      2.1±0.01ms    55.4 MB/sec    1.00   1910.0±5.33µs    61.5 MB/sec    1.12      2.1±0.00ms    54.7 MB/sec
encode/snap_uflat04_pdf        1.11   1695.1±0.19µs    57.6 MB/sec    1.00   1528.3±0.15µs    63.9 MB/sec    1.12   1705.8±0.16µs    57.3 MB/sec
encode/snap_uflat05_html4      5.10      4.4±0.00ms    89.7 MB/sec    1.00    854.4±0.08µs   457.2 MB/sec    1.12    954.2±0.11µs   409.4 MB/sec
encode/snap_uflat06_txt1       1.36      3.6±0.01ms    40.4 MB/sec    1.00      2.6±0.00ms    55.1 MB/sec    1.01      2.7±0.01ms    54.6 MB/sec
encode/snap_uflat07_txt2       1.34      3.1±0.01ms    38.5 MB/sec    1.01      2.3±0.00ms    51.2 MB/sec    1.00      2.3±0.00ms    51.8 MB/sec
encode/snap_uflat08_txt3       1.37      9.5±0.00ms    42.6 MB/sec    1.00      7.0±0.01ms    58.5 MB/sec    1.02      7.1±0.02ms    57.6 MB/sec
encode/snap_uflat09_txt4       1.34     12.3±0.04ms    37.3 MB/sec    1.00      9.3±0.00ms    49.7 MB/sec    1.00      9.2±0.00ms    49.9 MB/sec
encode/snap_uflat10_pb         1.95   1568.4±3.90µs    72.1 MB/sec    1.00    802.5±0.08µs   140.9 MB/sec    1.14    915.6±0.11µs   123.5 MB/sec
encode/snap_uflat11_gaviota    1.49      3.5±0.00ms    50.2 MB/sec    1.00      2.4±0.00ms    74.8 MB/sec    1.01      2.4±0.00ms    73.9 MB/sec
```

```
Key:
lzfse_ref: lzfse reference library
rust     : manic_lzfse
rust_ring: manic_lzfse ring

Column: 1 2 3
1: relative time                    lower is better, 1.00 is the fastest 
2: mean time ± standard deviation   lower is better
3: throughput                       higher is better
```

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.51.0`.


## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.


## Alternatives

* [lzfse-rs](https://github.com/citruz/lzfse-rs) - bindings to the reference [LZFSE](https://github.com/lzfse/lzfse) implementation.
