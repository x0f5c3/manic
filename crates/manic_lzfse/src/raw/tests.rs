use crate::kit::WIDE;
use crate::ops::Skip;
use crate::ring::{RingBlock, RingBox, RingReader, RingSize, RingType};
use crate::types::ByteReader;

use test_kit::Seq;

use super::block::RawBlock;
use super::ops;

#[derive(Copy, Clone, Debug)]
pub struct T;

impl RingSize for T {
    const RING_SIZE: u32 = 0x1000;
}

impl RingType for T {
    const RING_LIMIT: u32 = WIDE as u32;
}

impl RingBlock for T {
    const RING_BLK_SIZE: u32 = 0x0200;
}

const ABCD: [u8; 4] = [0x41, 0x42, 0x43, 0x44];

const ABCD_RAW: [u8; 12] = [0x62, 0x76, 0x78, 0x2d, 0x04, 0x00, 0x00, 0x00, 0x41, 0x42, 0x43, 0x44];

// Basic block 4 byte raw encoding test.
#[test]
fn block_enc_basic() -> crate::Result<()> {
    let mut enc = Vec::default();
    ops::raw_compress(&mut enc, ABCD.as_ref())?;
    assert_eq!(ABCD_RAW.as_ref(), &enc);
    Ok(())
}

// Basic block 4 byte raw decoding test.
#[test]
fn block_dec_basic() -> crate::Result<()> {
    let mut dec = Vec::default();
    let mut src = ABCD_RAW.as_ref();
    ops::raw_decompress(&mut dec, &mut src)?;
    assert_eq!(src.len(), 0);
    assert_eq!(ABCD.as_ref(), &dec);
    Ok(())
}

#[test]
#[ignore = "expensive"]
fn block_enc_dec_vec() -> crate::Result<()> {
    let src = Seq::default().take(65536).collect::<Vec<_>>();
    let mut dec = Vec::default();
    let mut enc = Vec::default();
    for n in 0..=src.len() {
        let src = &src[..n];
        enc.clear();
        ops::raw_compress(&mut enc, src)?;
        dec.clear();
        ops::raw_decompress(&mut dec, &mut enc.as_ref())?;
        assert_eq!(src, dec);
    }
    Ok(())
}

#[test]
#[ignore = "expensive"]
fn block_enc_dec_ring() -> crate::Result<()> {
    let mut ring_box = RingBox::<T>::default();
    let src = Seq::default().take(65536).collect::<Vec<_>>();
    let mut dec = Vec::default();
    let mut enc = Vec::default();
    for n in 0..src.len() {
        let src = &src[..n];
        enc.clear();
        ops::raw_compress(&mut enc, src)?;
        let mut enc = enc.as_slice();
        let mut rdr = RingReader::new((&mut ring_box).into(), &mut enc);
        dec.clear();
        ops::raw_decompress(&mut dec, &mut rdr)?;
        assert_eq!(src, dec);
    }
    Ok(())
}

#[test]
#[ignore = "expensive"]
fn block_enc_dec_ring_n() -> crate::Result<()> {
    let mut block = RawBlock::default();
    let mut ring_box = RingBox::<T>::default();
    let src = Seq::default().take(65536).collect::<Vec<_>>();
    let mut dec = Vec::default();
    let mut enc = Vec::default();
    ops::raw_compress(&mut enc, src.as_ref())?;
    for n in 1..src.len() {
        let mut enc = enc.as_slice();
        let mut rdr = RingReader::new((&mut ring_box).into(), &mut enc);
        dec.clear();
        rdr.fill()?;
        rdr.skip(block.load_short(rdr.view())? as usize);
        while block.decode_n(&mut dec, &mut rdr, n as u32)? {}
        assert_eq!(src, dec);
    }
    Ok(())
}
