// Empty/ zero byte files except for a small head pad, increasing in size.

macro_rules! test_pattern {
    ($name:ident, $encoder:expr) => {
        mod $name {
            use crate::buddy::Buddy;
            use crate::ops;

            use test_kit::Seq;

            use std::io;

            // PAD            | ZEROS
            // [0x00-0xFF]{u} | 0x00{v}
            fn gen(vec: &mut Vec<u8>, u: u32, v: u32) {
                vec.clear();
                let mut seq = Seq::default();
                for _ in 0..u {
                    vec.push(seq.gen());
                }
                vec.resize(u as usize + v as usize, 0);
            }

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_0() -> io::Result<()> {
                let mut vec = Vec::with_capacity(0x8000);
                let mut buddy = Buddy::default();
                for u in 1..=0x08 {
                    for v in 0..0x1000 {
                        gen(&mut vec, u, v);
                        buddy.encode_decode(&vec, $encoder)?;
                    }
                }
                Ok(())
            }

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_1() -> io::Result<()> {
                let mut vec = Vec::with_capacity(0x8000);
                let mut buddy = Buddy::default();
                for u in (0..=0x0008_0000).step_by(0x8000) {
                    for v in (0..=0x0008_0000).step_by(0x8000) {
                        gen(&mut vec, u, v);
                        buddy.encode_decode(&vec, $encoder)?;
                    }
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
