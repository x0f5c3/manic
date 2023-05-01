// No matching 4 byte sequences.

macro_rules! test_pattern {
    ($name:ident, $encoder:expr) => {
        mod $name {
            use crate::buddy::Buddy;
            use crate::ops;

            use test_kit::Useq;

            use std::io;

            #[test]
            fn encode_decode_0() -> io::Result<()> {
                let vec = Useq::default().take(0x0010_0000).collect::<Vec<_>>();
                Buddy::default().encode_decode(&vec, $encoder)?;
                Ok(())
            }
        }
    };
}

test_pattern!(encode, ops::encode);
test_pattern!(encode_bytes, ops::encode_bytes);
test_pattern!(encode_writer, ops::encode_writer);
test_pattern!(encode_writer_bytes, ops::encode_writer_bytes);
