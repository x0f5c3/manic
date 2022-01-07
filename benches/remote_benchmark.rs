use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use manic::Downloader;
use manic::Hash;
use std::io::Write;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::runtime::Builder;

async fn bench_remote(workers: u8) -> manic::Result<()> {
    let mut dl = Downloader::new(
        "https://github.com/schollz/croc/releases/download/v9.2.0/croc_9.2.0_Windows-64bit.zip",
        workers,
        None,
    )
    .await?;
    dl.verify(Hash::new_sha256(
        "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
    ));
    let _data = dl.download().await?;
    Ok(())
}

async fn bench_async(verify: bool) -> manic::Result<()> {
    let mut output = tokio::fs::File::from_std(tempfile::tempfile()?);
    let url =
        "https://github.com/schollz/croc/releases/download/v9.2.0/croc_9.2.0_Windows-64bit.zip";
    let resp = reqwest::get(url).await?.bytes().await?;
    output.write_all(resp.as_ref()).await?;
    if verify {
        let mut hash = Hash::new_sha256(
            "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
        );
        hash.update(resp.as_ref());
        hash.verify()?;
    }
    Ok(())
}

fn blocking_bench(verify: bool) -> manic::Result<()> {
    let mut output = tempfile::tempfile()?;
    let resp = reqwest::blocking::get(
        "https://github.com/schollz/croc/releases/download/v9.2.0/croc_9.2.0_Windows-64bit.zip",
    )?
    .bytes()?;
    output.write_all(resp.as_ref())?;
    if verify {
        let mut hash = Hash::new_sha256(
            "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
        );
        hash.update(resp.as_ref());
        hash.verify()?;
    }
    Ok(())
}

fn manic_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("manic_bench");
    for workers in 1..=10 {
        group.bench_with_input(
            BenchmarkId::new("manic_bench", workers),
            &(workers as u8),
            |b, s| {
                b.to_async(
                    Builder::new_multi_thread()
                        .worker_threads(10)
                        .enable_all()
                        .build()
                        .unwrap(),
                )
                .iter(|| bench_remote(*s))
            },
        );
    }
}

fn async_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_bench");
    group.bench_with_input(BenchmarkId::new("async_bench", true), &true, |b, s| {
        b.to_async(
            Builder::new_multi_thread()
                .worker_threads(10)
                .enable_all()
                .build()
                .unwrap(),
        )
        .iter(|| bench_async(*s))
    });
    group.bench_with_input(BenchmarkId::new("async_bench", false), &false, |b, s| {
        b.to_async(
            Builder::new_multi_thread()
                .worker_threads(10)
                .enable_all()
                .build()
                .unwrap(),
        )
        .iter(|| bench_async(*s))
    });
}

fn blocking(c: &mut Criterion) {
    let mut group = c.benchmark_group("blocking bench");
    group.bench_with_input(BenchmarkId::new("classic_bench", true), &true, |b, s| {
        b.iter(|| blocking_bench(*s))
    });
    group.bench_with_input(BenchmarkId::new("classic_bench", false), &false, |b, s| {
        b.iter(|| blocking_bench(*s))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(60)).sample_size(10);
    targets = manic_bench, async_bench, blocking
}
criterion_main!(benches);
