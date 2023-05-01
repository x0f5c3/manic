// Patchwork files long.

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
                let base = Iterator::take(Seq::default(), 0x0100).collect::<Vec<_>>();
                let mut vec = Vec::with_capacity(0x0040_0000);
                let mut buddy = Buddy::default();
                for seed in 0..0x0100 {
                    let mut rng = Rng::new(seed);
                    vec.extend_from_slice(&base);
                    for _ in 0..0x0001_0000 {
                        let top = vec.len();
                        let off = (rng.gen() >> 16) as usize % top;
                        let len = (rng.gen() >> 28) as usize % (top - off);
                        vec.resize(top + len, 0);
                        vec.copy_within(off..off + len, top);
                    }
                    buddy.encode_decode(&vec, $encoder)?;
                    vec.clear();
                }
                Ok(())
            }

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_1() -> io::Result<()> {
                let base = Iterator::take(Seq::default(), 0x0100).collect::<Vec<_>>();
                let mut vec = Vec::with_capacity(0x0040_0000);
                let mut buddy = Buddy::default();
                for seed in 0..0x0100 {
                    let mut rng = Rng::new(seed);
                    vec.extend_from_slice(&base);
                    for _ in 0..0x0001_0000 {
                        let top = vec.len();
                        let off = (rng.gen() >> 16) as usize % top;
                        let len = (rng.gen() >> 26) as usize % (top - off);
                        vec.resize(top + len, 0);
                        vec.copy_within(off..off + len, top);
                    }
                    buddy.encode_decode(&vec, $encoder)?;
                    vec.clear();
                }
                Ok(())
            }

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_2() -> io::Result<()> {
                let base = Iterator::take(Seq::default(), 0x0100).collect::<Vec<_>>();
                let mut vec = Vec::with_capacity(0x0040_0000);
                let mut buddy = Buddy::default();
                for seed in 0..0x0100 {
                    let mut rng = Rng::new(seed);
                    vec.extend_from_slice(&base);
                    for _ in 0..0x1000 {
                        let top = vec.len();
                        let off = (rng.gen() >> 16) as usize % top;
                        let len = (rng.gen() >> 22) as usize % (top - off);
                        vec.resize(top + len, 0);
                        vec.copy_within(off..off + len, top);
                    }
                    buddy.encode_decode(&vec, $encoder)?;
                    vec.clear();
                }
                Ok(())
            }

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_3() -> io::Result<()> {
                let base = Iterator::take(Seq::default(), 0x0100).collect::<Vec<_>>();
                let mut vec = Vec::with_capacity(0x0040_0000);
                let mut buddy = Buddy::default();
                for seed in 0..0x0100 {
                    let mut rng = Rng::new(seed);
                    vec.extend_from_slice(&base);
                    for _ in 0..0x0100 {
                        let top = vec.len();
                        let off = (rng.gen() >> 16) as usize % top;
                        let len = (rng.gen() >> 18) as usize % (top - off);
                        vec.resize(top + len, 0);
                        vec.copy_within(off..off + len, top);
                    }
                    buddy.encode_decode(&vec, $encoder)?;
                    vec.clear();
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
