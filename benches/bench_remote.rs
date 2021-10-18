use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use manic::Downloader;
use manic::Hash;
use std::time::Duration;
use tokio::runtime::Builder;

async fn bench_remote(workers: u8) -> manic::Result<()> {
    let mut dl = Downloader::new(
        "https://github.com/schollz/croc/releases/download/v9.2.0/croc_9.2.0_Windows-64bit.zip",
        workers,
    )
    .await?;
    dl.verify(Hash::SHA256(
        "0ac1e91826eabd78b1aca342ac11292a7399a2fdf714158298bae1d1bd12390b".to_string(),
    ));
    let _data = dl.download().await?;
    Ok(())
}

fn outer_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_remote");
    for workers in [1, 2, 3, 4, 5, 6].iter() {
        group.bench_with_input(
            BenchmarkId::new("bench_remote", workers),
            workers,
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

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20).measurement_time(Duration::from_secs(60));
    targets = outer_bench
}
criterion_main!(benches);
