use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode, Throughput};
use manic_lzfse::{self, LzfseDecoder, LzfseEncoder, LzfseRingDecoder, LzfseRingEncoder};
use std::time::Duration;

const SAMPLE_SIZE: usize = 20;
const MEASUREMENT_TIME: Duration = Duration::from_secs(20);

// Snappy benchmarks.

const CORPUS_HTML: &[u8] = include_bytes!("../data/snappy/html.lzfse");
const CORPUS_URLS_10K: &[u8] = include_bytes!("../data/snappy/urls.10K.lzfse");
const CORPUS_FIREWORKS: &[u8] = include_bytes!("../data/snappy/fireworks.jpeg.lzfse");
const CORPUS_PAPER_100K: &[u8] = include_bytes!("../data/snappy/paper-100k.pdf.lzfse");
const CORPUS_HTML_X_4: &[u8] = include_bytes!("../data/snappy/html_x_4.lzfse");
const CORPUS_ALICE29: &[u8] = include_bytes!("../data/snappy/alice29.txt.lzfse");
const CORPUS_ASYOULIK: &[u8] = include_bytes!("../data/snappy/asyoulik.txt.lzfse");
const CORPUS_LCET10: &[u8] = include_bytes!("../data/snappy/lcet10.txt.lzfse");
const CORPUS_PLRABN12: &[u8] = include_bytes!("../data/snappy/plrabn12.txt.lzfse");
const CORPUS_GEOPROTO: &[u8] = include_bytes!("../data/snappy/geo.protodata.lzfse");
const CORPUS_KPPKN: &[u8] = include_bytes!("../data/snappy/kppkn.gtb.lzfse");

// Synthetic benchmarks.

// Noise: stress literal/ null match vectors.
const SYNTH_RANDOM: &[u8] = include_bytes!("../data/synth/random.lzfse");

// Random words matches: stress short match run/ long distance vectors.
const SYNTH_WORD04: &[u8] = include_bytes!("../data/synth/word04.lzfse");
const SYNTH_WORD05: &[u8] = include_bytes!("../data/synth/word05.lzfse");
const SYNTH_WORD06: &[u8] = include_bytes!("../data/synth/word06.lzfse");
const SYNTH_WORD07: &[u8] = include_bytes!("../data/synth/word07.lzfse");
const SYNTH_WORD08: &[u8] = include_bytes!("../data/synth/word08.lzfse");
const SYNTH_WORD09: &[u8] = include_bytes!("../data/synth/word09.lzfse");
const SYNTH_WORD10: &[u8] = include_bytes!("../data/synth/word10.lzfse");
const SYNTH_WORD11: &[u8] = include_bytes!("../data/synth/word11.lzfse");
const SYNTH_WORD12: &[u8] = include_bytes!("../data/synth/word12.lzfse");
const SYNTH_WORD13: &[u8] = include_bytes!("../data/synth/word13.lzfse");
const SYNTH_WORD14: &[u8] = include_bytes!("../data/synth/word14.lzfse");
const SYNTH_WORD15: &[u8] = include_bytes!("../data/synth/word15.lzfse");
const SYNTH_WORD16: &[u8] = include_bytes!("../data/synth/word16.lzfse");
const SYNTH_WORD32: &[u8] = include_bytes!("../data/synth/word32.lzfse");
const SYNTH_WORD64: &[u8] = include_bytes!("../data/synth/word64.lzfse");

// Long fixed repeating sequences: stress long match/ short distance vectors.
const SYNTH_REPL01: &[u8] = include_bytes!("../data/synth/repl01.lzfse");
const SYNTH_REPL02: &[u8] = include_bytes!("../data/synth/repl02.lzfse");
const SYNTH_REPL03: &[u8] = include_bytes!("../data/synth/repl03.lzfse");
const SYNTH_REPL04: &[u8] = include_bytes!("../data/synth/repl04.lzfse");
const SYNTH_REPL05: &[u8] = include_bytes!("../data/synth/repl05.lzfse");
const SYNTH_REPL06: &[u8] = include_bytes!("../data/synth/repl06.lzfse");
const SYNTH_REPL07: &[u8] = include_bytes!("../data/synth/repl07.lzfse");
const SYNTH_REPL08: &[u8] = include_bytes!("../data/synth/repl08.lzfse");
const SYNTH_REPL09: &[u8] = include_bytes!("../data/synth/repl09.lzfse");
const SYNTH_REPL10: &[u8] = include_bytes!("../data/synth/repl10.lzfse");
const SYNTH_REPL11: &[u8] = include_bytes!("../data/synth/repl11.lzfse");
const SYNTH_REPL12: &[u8] = include_bytes!("../data/synth/repl12.lzfse");
const SYNTH_REPL13: &[u8] = include_bytes!("../data/synth/repl13.lzfse");
const SYNTH_REPL14: &[u8] = include_bytes!("../data/synth/repl14.lzfse");
const SYNTH_REPL15: &[u8] = include_bytes!("../data/synth/repl15.lzfse");
const SYNTH_REPL16: &[u8] = include_bytes!("../data/synth/repl16.lzfse");
const SYNTH_REPL32: &[u8] = include_bytes!("../data/synth/repl32.lzfse");
const SYNTH_REPL64: &[u8] = include_bytes!("../data/synth/repl64.lzfse");

// Short fixed repeating sequences: stress short match/ short distance vectors.
// Asymmetric encode/ decode.
const SYNTH_REPS04: &[u8] = include_bytes!("../data/synth/reps04.lzfse");
const SYNTH_REPS05: &[u8] = include_bytes!("../data/synth/reps05.lzfse");
const SYNTH_REPS06: &[u8] = include_bytes!("../data/synth/reps06.lzfse");
const SYNTH_REPS07: &[u8] = include_bytes!("../data/synth/reps07.lzfse");
const SYNTH_REPS08: &[u8] = include_bytes!("../data/synth/reps08.lzfse");
const SYNTH_REPS09: &[u8] = include_bytes!("../data/synth/reps09.lzfse");
const SYNTH_REPS10: &[u8] = include_bytes!("../data/synth/reps10.lzfse");
const SYNTH_REPS11: &[u8] = include_bytes!("../data/synth/reps11.lzfse");
const SYNTH_REPS12: &[u8] = include_bytes!("../data/synth/reps12.lzfse");
const SYNTH_REPS13: &[u8] = include_bytes!("../data/synth/reps13.lzfse");
const SYNTH_REPS14: &[u8] = include_bytes!("../data/synth/reps14.lzfse");
const SYNTH_REPS15: &[u8] = include_bytes!("../data/synth/reps15.lzfse");
const SYNTH_REPS16: &[u8] = include_bytes!("../data/synth/reps16.lzfse");
const SYNTH_REPS32: &[u8] = include_bytes!("../data/synth/reps32.lzfse");
const SYNTH_REPS64: &[u8] = include_bytes!("../data/synth/reps64.lzfse");
const SYNTH_REPSIN: &[u8] = include_bytes!("../data/synth/repsin.lzfse");

fn all(c: &mut Criterion) {
    #[cfg(feature = "lzfse_ref")]
    snappy(c, lzfse_ref_encode);
    #[cfg(feature = "lzfse_ref")]
    snappy(c, lzfse_ref_decode);
    snappy(c, rust_encode);
    snappy(c, rust_decode);
    snappy(c, rust_ring_encode);
    snappy(c, rust_ring_decode);

    #[cfg(feature = "lzfse_ref")]
    synth_random(c, lzfse_ref_encode);
    #[cfg(feature = "lzfse_ref")]
    synth_random(c, lzfse_ref_decode);
    synth_random(c, rust_encode);
    synth_random(c, rust_decode);
    synth_random(c, rust_ring_encode);
    synth_random(c, rust_ring_decode);

    #[cfg(feature = "lzfse_ref")]
    synth_word(c, lzfse_ref_encode);
    #[cfg(feature = "lzfse_ref")]
    synth_word(c, lzfse_ref_decode);
    synth_word(c, rust_encode);
    synth_word(c, rust_decode);
    synth_word(c, rust_ring_encode);
    synth_word(c, rust_ring_decode);

    #[cfg(feature = "lzfse_ref")]
    synth_repl(c, lzfse_ref_decode);
    synth_repl(c, rust_decode);
    synth_repl(c, rust_ring_decode);
}

/// Synthetic data
fn synth_random(c: &mut Criterion, mut engine: impl FnMut(&mut Criterion, &str, &[u8])) {
    engine(c, "synth_random", SYNTH_RANDOM);
}

/// Synthetic data
fn synth_word(c: &mut Criterion, mut engine: impl FnMut(&mut Criterion, &str, &[u8])) {
    engine(c, "synth_word04", SYNTH_WORD04);
    engine(c, "synth_word05", SYNTH_WORD05);
    engine(c, "synth_word06", SYNTH_WORD06);
    engine(c, "synth_word07", SYNTH_WORD07);
    engine(c, "synth_word08", SYNTH_WORD08);
    engine(c, "synth_word09", SYNTH_WORD09);
    engine(c, "synth_word10", SYNTH_WORD10);
    engine(c, "synth_word11", SYNTH_WORD11);
    engine(c, "synth_word12", SYNTH_WORD12);
    engine(c, "synth_word13", SYNTH_WORD13);
    engine(c, "synth_word14", SYNTH_WORD14);
    engine(c, "synth_word15", SYNTH_WORD15);
    engine(c, "synth_word16", SYNTH_WORD16);
    engine(c, "synth_word32", SYNTH_WORD32);
    engine(c, "synth_word64", SYNTH_WORD64);
}

/// Synthetic data
fn synth_repl(c: &mut Criterion, mut engine: impl FnMut(&mut Criterion, &str, &[u8])) {
    engine(c, "synth_repl01", SYNTH_REPL01);
    engine(c, "synth_repl02", SYNTH_REPL02);
    engine(c, "synth_repl03", SYNTH_REPL03);
    engine(c, "synth_repl04", SYNTH_REPL04);
    engine(c, "synth_repl05", SYNTH_REPL05);
    engine(c, "synth_repl06", SYNTH_REPL06);
    engine(c, "synth_repl07", SYNTH_REPL07);
    engine(c, "synth_repl08", SYNTH_REPL08);
    engine(c, "synth_repl09", SYNTH_REPL09);
    engine(c, "synth_repl10", SYNTH_REPL10);
    engine(c, "synth_repl11", SYNTH_REPL11);
    engine(c, "synth_repl12", SYNTH_REPL12);
    engine(c, "synth_repl13", SYNTH_REPL13);
    engine(c, "synth_repl14", SYNTH_REPL14);
    engine(c, "synth_repl15", SYNTH_REPL15);
    engine(c, "synth_repl16", SYNTH_REPL16);
    engine(c, "synth_repl32", SYNTH_REPL32);
    engine(c, "synth_repl64", SYNTH_REPL64);
    engine(c, "synth_reps04", SYNTH_REPS04);
    engine(c, "synth_reps05", SYNTH_REPS05);
    engine(c, "synth_reps06", SYNTH_REPS06);
    engine(c, "synth_reps07", SYNTH_REPS07);
    engine(c, "synth_reps08", SYNTH_REPS08);
    engine(c, "synth_reps09", SYNTH_REPS09);
    engine(c, "synth_reps10", SYNTH_REPS10);
    engine(c, "synth_reps11", SYNTH_REPS11);
    engine(c, "synth_reps12", SYNTH_REPS12);
    engine(c, "synth_reps13", SYNTH_REPS13);
    engine(c, "synth_reps14", SYNTH_REPS14);
    engine(c, "synth_reps15", SYNTH_REPS15);
    engine(c, "synth_reps16", SYNTH_REPS16);
    engine(c, "synth_reps32", SYNTH_REPS32);
    engine(c, "synth_reps64", SYNTH_REPS64);
    engine(c, "synth_repsin", SYNTH_REPSIN);
}

/// Snappy data
#[rustfmt::skip]
fn snappy(c: &mut Criterion, mut engine: impl FnMut(&mut Criterion, &str, &[u8])) {
    engine(c, "snap_uflat00_html",    CORPUS_HTML);
    engine(c, "snap_uflat01_urls",    CORPUS_URLS_10K);
    engine(c, "snap_uflat02_jpg",     CORPUS_FIREWORKS);
    engine(c, "snap_uflat04_pdf",     CORPUS_PAPER_100K);
    engine(c, "snap_uflat05_html4",   CORPUS_HTML_X_4);
    engine(c, "snap_uflat06_txt1",    CORPUS_ALICE29);
    engine(c, "snap_uflat07_txt2",    CORPUS_ASYOULIK);
    engine(c, "snap_uflat08_txt3",    CORPUS_LCET10);
    engine(c, "snap_uflat09_txt4",    CORPUS_PLRABN12);
    engine(c, "snap_uflat10_pb",      CORPUS_GEOPROTO);
    engine(c, "snap_uflat11_gaviota", CORPUS_KPPKN);
}

fn rust_encode(c: &mut Criterion, tag: &str, enc: &[u8]) {
    let mut encoder = LzfseEncoder::default();
    encode(c, "rust", tag, enc, |src, dst| {
        dst.clear();
        encoder.encode_bytes(src, dst).expect("encode error");
    })
}

fn rust_decode(c: &mut Criterion, tag: &str, enc: &[u8]) {
    let mut decoder = LzfseDecoder::default();
    decode(c, "rust", tag, enc, |src, dst| {
        dst.clear();
        decoder.decode_bytes(src, dst).expect("decode error");
    })
}

fn rust_ring_encode(c: &mut Criterion, tag: &str, enc: &[u8]) {
    let mut encoder = LzfseRingEncoder::default();
    encode(c, "rust_ring", tag, enc, |mut src, dst| {
        dst.clear();
        encoder.encode(&mut src, dst).expect("encode error");
    })
}

fn rust_ring_decode(c: &mut Criterion, tag: &str, enc: &[u8]) {
    let mut decoder = LzfseRingDecoder::default();
    decode(c, "rust_ring", tag, enc, |mut src, dst| {
        dst.clear();
        decoder.decode(&mut src, dst).expect("decode error");
    })
}

#[cfg(feature = "lzfse_ref")]
fn lzfse_ref_encode(c: &mut Criterion, tag: &str, enc: &[u8]) {
    encode(c, "lzfse_ref", tag, enc, |src, dst| {
        assert_ne!(lzfse_sys::encode(src, dst.as_mut_slice()), 0);
    })
}

#[cfg(feature = "lzfse_ref")]
fn lzfse_ref_decode(c: &mut Criterion, tag: &str, enc: &[u8]) {
    decode(c, "lzfse_ref", tag, enc, |src, dst| {
        assert_ne!(lzfse_sys::decode(src, dst.as_mut_slice()), 0);
    })
}

fn encode(
    c: &mut Criterion,
    engine: &str,
    tag: &str,
    enc: &[u8],
    f: impl FnMut(&[u8], &mut Vec<u8>),
) {
    let dec = decode_bytes(enc);
    let len = dec.len();
    let mut enc = vec![0u8; enc.len() + 4096];
    let mut bench_name: String = "encode/".to_owned();
    bench_name.push_str(tag);
    execute(c, engine, &bench_name, &dec, &mut enc, len, f);
}

fn decode(
    c: &mut Criterion,
    engine: &str,
    tag: &str,
    enc: &[u8],
    f: impl FnMut(&[u8], &mut Vec<u8>),
) {
    let mut dec = decode_bytes(enc);
    let len = dec.len();
    let mut bench_name: String = "decode/".to_owned();
    bench_name.push_str(tag);
    execute(c, engine, &bench_name, enc, &mut dec, len, f);
}

fn execute(
    c: &mut Criterion,
    engine: &str,
    bench_name: &str,
    src: &[u8],
    dst: &mut Vec<u8>,
    len: usize,
    mut f: impl FnMut(&[u8], &mut Vec<u8>),
) {
    let mut group = c.benchmark_group(engine);
    group.measurement_time(MEASUREMENT_TIME);
    group.sample_size(SAMPLE_SIZE);
    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Bytes(len as u64));
    group.bench_function(bench_name, |b| b.iter(|| f(black_box(src), black_box(dst))));
    group.finish();
}

fn decode_bytes(enc: &[u8]) -> Vec<u8> {
    let mut dec = Vec::default();
    manic_lzfse::decode_bytes(enc, &mut dec).expect("decode error");
    dec
}

criterion_group!(benches, all);
criterion_main!(benches);
