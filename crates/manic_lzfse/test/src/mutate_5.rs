// Compound random byte mutation. We are looking to break the decoder. It should not hang/ segfault/
// panic/ trip debug assertions or break in a any other fashion.

const VX1: &[u8] = include_bytes!("../../data/mutate/vx1.lzfse");
const VX2: &[u8] = include_bytes!("../../data/mutate/vx2.lzfse");
const VXN: &[u8] = include_bytes!("../../data/mutate/vxn.lzfse");
const RAW: &[u8] = include_bytes!("../../data/mutate/raw.lzfse");

const VX1_HASH: &[u8] = include_bytes!("../../data/mutate/vx1.hash");
const VX2_HASH: &[u8] = include_bytes!("../../data/mutate/vx2.hash");
const VXN_HASH: &[u8] = include_bytes!("../../data/mutate/vxn.hash");
const RAW_HASH: &[u8] = include_bytes!("../../data/mutate/raw.hash");

macro_rules! test_mutate {
    ($name:ident, $data:ident, $hash:ident) => {
        mod $name {
            use crate::buddy::Buddy;
            use crate::ops;

            use manic_lzfse::LzfseRingDecoder;
            use test_kit::Rng;

            use std::io;

            pub fn check_mutate<F>(data: &[u8], hash: &[u8], decode: F) -> io::Result<()>
            where
                F: Fn(&mut LzfseRingDecoder, &[u8], &mut Vec<u8>) -> io::Result<()>,
            {
                let mut buddy = Buddy::default();
                for seed in 0..0x100 {
                    let mut rng = Rng::new(seed);
                    let mut data = data.to_vec();
                    for _ in 0..0x100 {
                        let n = rng.gen() % data.len() as u32;
                        let index = n as usize / 8;
                        let byte = rng.gen() as u8;
                        data[index] ^= byte;
                        let _ = buddy.blind_decode(&data, &decode);
                    }
                }
                buddy.decode_hash(&data, hash, ops::decode)
            }

            #[test]
            #[ignore = "expensive"]
            fn mutate() -> io::Result<()> {
                check_mutate(super::$data, super::$hash, ops::decode)
            }

            #[test]
            #[ignore = "expensive"]
            fn mutate_bytes() -> io::Result<()> {
                check_mutate(super::$data, super::$hash, ops::decode_bytes)
            }

            #[test]
            #[ignore = "expensive"]
            fn mutate_reader() -> io::Result<()> {
                check_mutate(super::$data, super::$hash, ops::decode_reader)
            }

            #[test]
            #[ignore = "expensive"]
            fn mutate_reader_bytes() -> io::Result<()> {
                check_mutate(super::$data, super::$hash, ops::decode_reader_bytes)
            }
        }
    };
}

test_mutate!(raw, RAW, RAW_HASH);
test_mutate!(vxn, VXN, VXN_HASH);
test_mutate!(vx1, VX1, VX1_HASH);
test_mutate!(vx2, VX2, VX2_HASH);
