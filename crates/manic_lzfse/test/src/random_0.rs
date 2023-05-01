// Random low entropy, mask: 0x01010101.

macro_rules! test_pattern {
    ($name:ident, $encoder:expr) => {
        mod $name {
            use crate::buddy::Buddy;
            use crate::ops;

            use test_kit::{Rng, Seq};

            use std::io;

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_0() -> io::Result<()> {
                let mut buddy = Buddy::default();
                for seed in 0..0x0080 {
                    let vec = Iterator::take(Seq::masked(Rng::new(seed), 0x01010101), 0x0010_0000)
                        .collect::<Vec<_>>();
                    buddy.encode_decode(&vec, $encoder)?;
                }
                Ok(())
            }

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_1() -> io::Result<()> {
                let mut buddy = Buddy::default();
                for seed in 0..0x0800 {
                    let vec = Iterator::take(Seq::masked(Rng::new(seed), 0x01010101), 0x1000)
                        .collect::<Vec<_>>();
                    buddy.encode_decode(&vec, $encoder)?;
                }
                Ok(())
            }
        }
    };
}

test_pattern!(encode, ops::encode);
test_pattern!(encode_bytes, ops::encode_bytes);
test_pattern!(encode_writer, ops::encode_writer);
test_pattern!(encode_writer_bytes, ops::encode_writer_bytes);
