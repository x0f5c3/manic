// Increasing length mutation. We are looking to break the decoder. It should not hang/ segfault/
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

            use std::io::{self, Write};

            pub fn check_mutate<F>(data: &[u8], hash: &[u8], decode: F) -> io::Result<()>
            where
                F: Fn(&mut LzfseRingDecoder, &[u8], &mut Vec<u8>) -> io::Result<()>,
            {
                let mut buddy = Buddy::default();
                let mut twin = Vec::with_capacity(data.len() * 2);
                twin.write_all(data)?;
                twin.write_all(data)?;
                for index in (data.len() + 1..twin.len()).rev() {
                    assert!(buddy.decode_hash(&twin[..index], hash, &decode).is_err());
                }
                Ok(())
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
