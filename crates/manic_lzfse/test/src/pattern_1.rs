// Empty/ zero byte files, increasing in size.

macro_rules! test_pattern {
    ($name:ident, $encoder:expr) => {
        mod $name {
            use crate::buddy::Buddy;
            use crate::ops;

            use std::io;

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_0() -> io::Result<()> {
                let mut vec = Vec::with_capacity(0x8000);
                let mut buddy = Buddy::default();
                while vec.len() != 0x8000 {
                    buddy.encode_decode(&vec, $encoder)?;
                    vec.push(0);
                }
                Ok(())
            }

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_1() -> io::Result<()> {
                let mut vec = Vec::with_capacity(0x0010_0000);
                let mut buddy = Buddy::default();
                while vec.len() != 0x0008_0200 {
                    buddy.encode_decode(&vec, $encoder)?;
                    vec.extend_from_slice(&[0u8; 0x100]);
                }
                Ok(())
            }

            #[test]
            #[ignore = "expensive"]
            fn encode_decode_2() -> io::Result<()> {
                let mut vec = Vec::with_capacity(0x0010_0000);
                vec.resize(0x0007_FE00, 0);
                let mut buddy = Buddy::default();
                while vec.len() != 0x0008_0200 {
                    buddy.encode_decode(&vec, $encoder)?;
                    vec.push(0);
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
