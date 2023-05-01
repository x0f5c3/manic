// Random length writes.

use manic_lzfse::LzfseRingEncoder;
use test_kit::{Rng, Seq};

use std::io::{self, Write};

fn check(multiplier: usize, iterations: u32) -> io::Result<()> {
    let data = Seq::default().take(0x0020_0000).collect::<Vec<_>>();
    let mut enc_base = Vec::default();
    let mut encoder = LzfseRingEncoder::default();
    encoder.encode(&mut data.as_slice(), &mut enc_base)?;
    let mut enc = Vec::with_capacity(data.len() + 0x0200);
    for seed in 0..iterations {
        let mut encoder = LzfseRingEncoder::default();

        let mut copy = data.as_slice();
        let mut wtr = encoder.writer(&mut enc);
        let mut rng = Rng::new(seed);
        while !copy.is_empty() {
            let n = ((rng.gen() as usize % 0x20) * multiplier).min(copy.len());
            let bytes = &copy[..n];
            copy = &copy[n..];
            wtr.write_all(bytes)?;
        }
        wtr.finalize()?;
        assert!(enc == enc_base);
        enc.clear();
    }
    Ok(())
}

#[test]
#[ignore = "expensive"]
fn encode_decode_0() -> io::Result<()> {
    check(1, 0x0100)
}

#[test]
#[ignore = "expensive"]
fn encode_decode_1() -> io::Result<()> {
    check(0x10, 0x0100)
}

#[test]
#[ignore = "expensive"]
fn encode_decode_2() -> io::Result<()> {
    check(0x0100, 0x0100)
}

#[test]
#[ignore = "expensive"]
fn encode_decode_3() -> io::Result<()> {
    check(0x1000, 0x0100)
}

#[test]
#[ignore = "expensive"]
fn encode_decode_4() -> io::Result<()> {
    check(0x0001_0000, 0x0100)
}
