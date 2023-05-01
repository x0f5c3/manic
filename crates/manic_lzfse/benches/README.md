# bench

[`Criterion`](https://github.com/bheisler/criterion.rs) powered benchmarks.

## Basic usage

```bash
$ RUSTFLAGS="-C opt-level=3 -C target-cpu=native -C codegen-units=1" cargo bench snap --manifest-path bench/Cargo.toml 
```

```bash
$ RUSTFLAGS="-C opt-level=3 -C target-cpu=native -C codegen-units=1" cargo bench snap --manifest-path bench/Cargo.toml -- --save-baseline before
```

## Lzfse reference

Benchmark the reference `lzfse` library.

For this to work, we need to build the reference LZFSE `liblzfse.a` library and inform `rustc` of it's whereabouts. See the `lzfse_sys/README.md` for instructions on how to do this.

```bash
$ RUSTFLAGS="-L /usr/local/lib/x86_64-linux-gnu -C opt-level=3 -C target-cpu=native -C codegen-units=1" cargo bench snap --manifest-path bench/Cargo.toml --features lzfse_ref
```
## Organization

The benchmarks are organized by: engine, operation and dataset.

* engine: lzfse_ref, rust, rust_ring.

* operation: encode, decode.

* dataset: snappy, synth

Output is formatted as: engine/operation/dataset_data

As a matter of expedience the [`snappy`](https://github.com/google/snappy) data is used as a generalized set and is our primary reference. As an alternative the synth(etic) data is comprised of noise/ naive patterns and is useful in tuning internal components.

## Critcmp

To compare benchmarks we can use [`critcmp`](https://github.com/BurntSushi/critcmp).

Baseline `new` compare all engines.

```bash
$ critcmp new -g '.*?/(.*$)'
```

Baseline `new` compare `rust` with `rust_ring`.

```bash
$ critcmp new -g '[t|g]/(.*$)'
```

Baseline `new` compare `lzfse_ref` with `rust`.

```bash
$ critcmp new -g '[f|t]/(.*$)'
```

Baseline `new` compare `lzfse_ref` with `rust_ring`.

```bash
$ critcmp new -g '[f|g]/(.*$)'
```
