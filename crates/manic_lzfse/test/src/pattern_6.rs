// Random short repeating sequences.

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
                let literals = Seq::default().take(0x4000).collect::<Vec<_>>();
                let mut data = Vec::default();
                let mut buddy = Buddy::default();
                for seed in 0..0x10 {
                    data.clear();
                    let mut rng = Rng::new(seed);
                    let mut literals = literals.as_slice();
                    while literals.len() != 0 {
                        let l = rng.gen() as usize % 0x20 + 1;
                        let l = l.min(literals.len());
                        data.extend_from_slice(&literals[..l]);
                        literals = &literals[l..];
                        let m = rng.gen() as usize % 0x0100;
                        for _ in 0..m {
                            let b = data[data.len() - l];
                            data.push(b);
                        }
                        buddy.encode_decode(&data, $encoder)?;
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
