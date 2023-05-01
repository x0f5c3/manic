// Random length reads.

use manic_lzfse::{LzfseEncoder, LzfseRingDecoder};
use test_kit::{Rng, Seq};

use std::io::{self};

fn check(multiplier: usize, iterations: u32) -> io::Result<()> {
    let data = Seq::default().take(0x0080_0000).collect::<Vec<_>>();
    let mut enc = Vec::default();
    LzfseEncoder::default().encode_bytes(&data, &mut enc)?;
    let mut decoder = LzfseRingDecoder::default();
    let mut dec = Vec::with_capacity(data.len());
    for seed in 0..iterations {
        use io::Read;
        let mut rdr = decoder.reader(enc.as_slice());
        let mut rng = Rng::new(seed);
        loop {
            let n = (rng.gen() as usize % 0x20) * multiplier;
            let i = dec.len();
            dec.resize(i + n, 0);
            let m = rdr.read(&mut dec[i..i + n])?;
            if n != m {
                dec.truncate(i + m);
                break;
            }
        }
        rdr.into_inner();
        assert!(dec == data);
        dec.clear();
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
