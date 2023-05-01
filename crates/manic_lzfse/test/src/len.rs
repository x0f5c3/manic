use manic_lzfse::{LzfseDecoder, LzfseEncoder, LzfseRingDecoder, LzfseRingEncoder};
use test_kit::{Rng, Seq};

use std::io;

#[allow(clippy::needless_range_loop)]
#[test]
#[ignore = "expensive"]
fn len() -> io::Result<()> {
    let mut data = vec![0u8; 0x8000];
    let mut enc = Vec::with_capacity(0x8800);
    let mut dec = Vec::with_capacity(0x8000);
    let mut encoder = LzfseEncoder::default();
    let mut decoder = LzfseDecoder::default();
    for n in 0..0x4000 {
        let mut seq = Seq::masked(Rng::new(n), 0x0000_0303);
        for i in 0..n as usize {
            data[i] = seq.gen();
        }
        let v = encoder.encode_bytes(&data[..n as usize], &mut enc)?;
        assert_eq!(enc.len() as u64, v);
        let v = decoder.decode_bytes(&enc, &mut dec)?;
        assert_eq!(dec.len() as u64, v);
        enc.clear();
        dec.clear();
    }
    Ok(())
}

#[allow(clippy::needless_range_loop)]
#[test]
#[ignore = "expensive"]
fn ring_len() -> io::Result<()> {
    let mut data = vec![0u8; 0x8000];
    let mut enc = Vec::with_capacity(0x8800);
    let mut dec = Vec::with_capacity(0x8000);
    let mut encoder = LzfseRingEncoder::default();
    let mut decoder = LzfseRingDecoder::default();
    for n in 0..0x4000 {
        let mut seq = Seq::masked(Rng::new(n), 0x0000_0303);
        for i in 0..n as usize {
            data[i] = seq.gen();
        }
        let mut src = &data[..n as usize];
        let (u, v) = encoder.encode(&mut src, &mut enc)?;
        assert_eq!(src.len(), 0);
        assert_eq!(n as u64, u);
        assert_eq!(enc.len() as u64, v);
        let mut src = enc.as_slice();
        let (u, v) = decoder.decode(&mut src, &mut dec)?;
        assert_eq!(src.len(), 0);
        assert_eq!(enc.len() as u64, u);
        assert_eq!(dec.len() as u64, v);
        enc.clear();
        dec.clear();
    }
    Ok(())
}
