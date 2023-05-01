// Max/ min double word mutation. We are looking to break the decoder. It should not hang/ segfault/
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

            use std::io;

            pub fn check_mutate<F>(data: &[u8], hash: &[u8], decode: F) -> io::Result<()>
            where
                F: Fn(&mut LzfseRingDecoder, &[u8], &mut Vec<u8>) -> io::Result<()>,
            {
                let mut buddy = Buddy::default();
                let mut data = data.to_vec();
                for index in 0..data.len() - 3 {
                    let u = data[index + 0];
                    let v = data[index + 1];
                    let w = data[index + 2];
                    let x = data[index + 3];
                    data[index + 0] = 0x00;
                    data[index + 1] = 0x00;
                    data[index + 2] = 0x00;
                    data[index + 3] = 0x00;
                    let _ = buddy.blind_decode(&data, &decode);
                    data[index + 0] = 0xFF;
                    data[index + 1] = 0xFF;
                    data[index + 2] = 0xFF;
                    data[index + 3] = 0xFF;
                    let _ = buddy.blind_decode(&data, &decode);
                    data[index + 0] = u;
                    data[index + 1] = v;
                    data[index + 2] = w;
                    data[index + 3] = x;
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
