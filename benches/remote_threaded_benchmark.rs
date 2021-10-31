use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use manic::threaded::Downloader;
use manic::threaded::Hash;
use std::time::Duration;

fn bench_remote(workers: u8, verify: bool) -> manic::threaded::Result<()> {
    let mut dl = Downloader::new(
        "https://github.com/schollz/croc/releases/download/v9.2.0/croc_9.2.0_Windows-64bit.zip",
        workers,
    )?;
    if verify {
        dl.verify(Hash::new_sha256(
            "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
        ));
    }
    let _data = dl.download()?;
    Ok(())
}

fn manic_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("manic_threaded_bench");
    let input_vec: Vec<u8> = (1..=40).collect();
    for workers in input_vec {
        group.bench_with_input(
            BenchmarkId::new("manic_threaded_bench verified", workers),
            &workers,
            |b, s| b.iter(|| bench_remote(*s, true)),
        );
        group.bench_with_input(
            BenchmarkId::new("manic_threaded_bench", workers),
            &workers,
            |b, s| b.iter(|| bench_remote(*s, false)),
        );
    }
}
criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(30)).sample_size(10);
    targets = manic_bench
}
criterion_main!(benches);
