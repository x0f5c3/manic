use crate::bits::{BitDst, BitReader, BitSrc, BitWriter};
use crate::kit::{CopyTypeIndex, WIDE};
use crate::lmd::LMax;
use crate::types::ShortBuffer;

use super::block::LiteralParam;
use super::constants::*;
use super::decoder::{self, Decoder};
use super::encoder::{self, Encoder};
use super::error_kind::FseErrorKind;
use super::Fse;

use std::io;
use std::usize;

const BUF_LEN: usize = LITERALS_PER_BLOCK as usize + MAX_L_VALUE as usize + WIDE;

#[repr(C)]
pub struct Literals(Box<[u8]>, pub usize);

impl Literals {
    #[inline(always)]
    pub unsafe fn push_unchecked_max<I>(&mut self, literals: &mut I)
    where
        I: ShortBuffer,
    {
        assert!(Fse::MAX_LITERAL_LEN as u32 <= I::SHORT_LIMIT);
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        debug_assert!(self.1 + Fse::MAX_LITERAL_LEN as usize <= LITERALS_PER_BLOCK as usize);
        let ptr = self.0.as_mut_ptr().add(self.1);
        literals.read_short_raw::<CopyTypeIndex>(ptr, Fse::MAX_LITERAL_LEN as usize);
        self.1 += Fse::MAX_LITERAL_LEN as usize;
    }

    #[inline(always)]
    pub unsafe fn push_unchecked<I>(&mut self, literals: &mut I, n_literals: u32)
    where
        I: ShortBuffer,
    {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        debug_assert!(self.1 + n_literals as usize <= LITERALS_PER_BLOCK as usize);
        debug_assert!(n_literals <= I::SHORT_LIMIT);
        let ptr = self.0.as_mut_ptr().add(self.1);
        literals.read_short_raw::<CopyTypeIndex>(ptr, n_literals as usize);
        self.1 += n_literals as usize;
    }

    #[allow(clippy::clippy::identity_op)]
    pub fn load<T>(&mut self, src: T, decoder: &Decoder, param: &LiteralParam) -> crate::Result<()>
    where
        T: BitSrc,
    {
        let mut reader = BitReader::new(src, param.bits() as usize)?;
        let state = param.state();
        let mut state = (
            unsafe { decoder::U::new(state[0] as usize) },
            unsafe { decoder::U::new(state[1] as usize) },
            unsafe { decoder::U::new(state[2] as usize) },
            unsafe { decoder::U::new(state[3] as usize) },
        );
        let ptr = self.0.as_mut_ptr().cast::<u8>();
        let n_literals = param.num() as usize;
        debug_assert!(n_literals <= LITERALS_PER_BLOCK as usize);
        let mut i = 0;
        while i != n_literals {
            // `flush` constraints:
            // 32 bit systems: maximum of x2 10 bit pushes.
            // 64 bit systems: maximum of x5 10 bit pushes (although we only push 4 for simplicity).
            unsafe { *ptr.add(i + 0) = decoder.u(&mut reader, &mut state.0) };
            unsafe { *ptr.add(i + 1) = decoder.u(&mut reader, &mut state.1) };
            #[cfg(target_pointer_width = "32")]
            reader.flush();
            unsafe { *ptr.add(i + 2) = decoder.u(&mut reader, &mut state.2) };
            unsafe { *ptr.add(i + 3) = decoder.u(&mut reader, &mut state.3) };
            reader.flush();
            i += 4;
        }
        reader.finalize()?;
        if state
            != (
                decoder::U::default(),
                decoder::U::default(),
                decoder::U::default(),
                decoder::U::default(),
            )
        {
            return Err(FseErrorKind::BadLmdPayload.into());
        }
        self.1 = n_literals;
        Ok(())
    }

    pub fn store<T>(&self, dst: &mut T, encoder: &Encoder) -> io::Result<LiteralParam>
    where
        T: BitDst,
    {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        let mark = dst.pos();
        let n_literals = (self.1 + 3) / 4 * 4;
        let n_bytes = (n_literals * MAX_U_BITS as usize + 7) / 8;
        let mut writer = BitWriter::new(dst, n_bytes)?;
        let mut state = (
            encoder::U::default(),
            encoder::U::default(),
            encoder::U::default(),
            encoder::U::default(),
        );
        let ptr = self.0.as_ptr();
        let mut i = n_literals;
        while i != 0 {
            // `flush` constraints:
            // 32 bit systems: maximum of x2 10 bit pushes.
            // 64 bit systems: maximum of x5 10 bit pushes (although we only push 4 for simplicity).
            unsafe { encoder.u(&mut writer, &mut state.3, *ptr.add(i - 1)) };
            unsafe { encoder.u(&mut writer, &mut state.2, *ptr.add(i - 2)) };
            #[cfg(target_pointer_width = "32")]
            writer.flush();
            unsafe { encoder.u(&mut writer, &mut state.1, *ptr.add(i - 3)) };
            unsafe { encoder.u(&mut writer, &mut state.0, *ptr.add(i - 4)) };
            writer.flush();
            i -= 4;
        }
        let state = [
            u32::from(state.0) as u16,
            u32::from(state.1) as u16,
            u32::from(state.2) as u16,
            u32::from(state.3) as u16,
        ];
        let bits = writer.finalize()? as u32;
        let n_payload_bytes = (dst.pos() - mark) as u32;
        let n_literals = (self.1 as u32 + 3) / 4 * 4;
        Ok(LiteralParam::new(n_literals, n_payload_bytes, bits, state).expect("internal error"))
    }

    #[inline(always)]
    pub fn pad(&mut self) {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        self.pad_u(unsafe { *self.0.get(0) });
    }

    #[inline(always)]
    pub fn pad_u(&mut self, u: u8) {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        unsafe { self.0.get_mut(self.1..).get_mut(..4) }.fill(u);
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        self.1
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        self.1 = 0;
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
}

impl AsRef<[u8]> for Literals {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        unsafe { self.0.get(..self.1) }
    }
}

impl Default for Literals {
    fn default() -> Self {
        Self(vec![0u8; BUF_LEN].into_boxed_slice(), 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::bits::ByteBits;
    use crate::fse::Weights;

    use test_kit::{Rng, Seq};

    use super::*;

    /// Test buddy.
    struct Buddy {
        weights: Weights,
        encoder: Encoder,
        decoder: Decoder,
        src: Literals,
        dst: Literals,
        param: LiteralParam,
        enc: Vec<u8>,
        n_literals: usize,
    }

    impl Buddy {
        #[allow(dead_code)]
        pub fn push(&mut self, mut literals: &[u8]) {
            self.src.reset();
            self.n_literals = literals.len();
            assert!(self.n_literals <= LITERALS_PER_BLOCK as usize);
            unsafe { self.src.push_unchecked(&mut literals, self.n_literals as u32) }
            assert_eq!(literals.len(), 0);
        }

        fn encode(&mut self) -> io::Result<()> {
            let u = self.weights.load(&[], self.src.as_ref());
            self.src.pad_u(u);
            self.encoder.init(&self.weights);
            self.enc.clear();
            self.enc.resize(8, 0);
            self.param = self.src.store(&mut self.enc, &self.encoder)?;
            assert_eq!(self.enc.len(), 8 + self.param.n_payload_bytes() as usize);
            Ok(())
        }

        fn decode(&mut self) -> io::Result<()> {
            self.decoder.init(&self.weights);
            self.dst.load(ByteBits::new(&self.enc), &self.decoder, &self.param)?;
            Ok(())
        }

        fn check(&self) -> bool {
            assert!(self.n_literals as usize <= self.src.len());
            assert!(self.n_literals as usize <= self.dst.len());
            self.src.as_ref()[..self.n_literals] == self.dst.as_ref()[..self.n_literals]
        }

        fn check_encode_decode(&mut self, literals: &[u8]) -> io::Result<bool> {
            self.push(literals);
            self.encode()?;
            self.decode()?;
            Ok(self.check())
        }
    }

    impl Default for Buddy {
        fn default() -> Self {
            Self {
                weights: Weights::default(),
                encoder: Encoder::default(),
                decoder: Decoder::default(),
                src: Literals::default(),
                dst: Literals::default(),
                param: LiteralParam::default(),
                enc: Vec::default(),
                n_literals: 0,
            }
        }
    }

    #[test]
    fn empty() -> io::Result<()> {
        let mut buddy = Buddy::default();
        assert!(buddy.check_encode_decode(&[])?);
        Ok(())
    }

    #[test]
    #[ignore = "expensive"]
    fn incremental() -> io::Result<()> {
        let bytes = Seq::default().take(LITERALS_PER_BLOCK as usize + 1).collect::<Vec<_>>();
        let mut buddy = Buddy::default();
        for literal_len in 1..bytes.len() {
            assert!(buddy.check_encode_decode(&bytes[..literal_len])?);
        }
        Ok(())
    }

    // Random literals.
    #[test]
    #[ignore = "expensive"]
    fn rng_1() -> io::Result<()> {
        let mut bytes = vec![0; LITERALS_PER_BLOCK as usize];
        let mut buddy = Buddy::default();
        for literal_len in 0..bytes.len() {
            bytes.clear();
            Seq::new(Rng::new(literal_len as u32)).take(literal_len).for_each(|u| bytes.push(u));
            assert!(buddy.check_encode_decode(&bytes[..literal_len])?);
        }
        Ok(())
    }

    // Random literals, incremental entropy.
    #[test]
    #[ignore = "expensive"]
    fn rng_2() -> io::Result<()> {
        let mut bytes = vec![0; 0x1000];
        let mut buddy = Buddy::default();
        for entropy in 0..0xFF {
            let mask = entropy * 0x0101_0101;
            for literal_len in 0..bytes.len() {
                bytes.clear();
                Seq::masked(Rng::new(literal_len as u32), mask)
                    .take(literal_len)
                    .for_each(|u| bytes.push(u));
                assert!(buddy.check_encode_decode(&bytes[..literal_len])?);
            }
        }
        Ok(())
    }

    // Bitwise mutation. We are looking to break the decoder. In all cases the
    // decoder should reject invalid data via `Err(error)` and exit gracefully. It should not hang/
    // segfault/ panic/ trip debug assertions or break in a any other fashion.
    #[test]
    #[ignore = "expensive"]
    fn mutate_1() -> io::Result<()> {
        let mut buddy = Buddy::default();
        let mut bytes = Vec::default();
        for seed in 0..0x0100 {
            bytes.clear();
            Seq::new(Rng::new(seed)).take(0x1000).for_each(|u| bytes.push(u));
            assert!(buddy.check_encode_decode(&bytes)?);
            for index in 0..buddy.enc.len() {
                for n_bit in 0..8 {
                    let bit = 1 << n_bit;
                    buddy.enc[index] ^= bit;
                    let _ = buddy.decode();
                    buddy.enc[index] ^= bit;
                }
            }
            assert!(buddy.check_encode_decode(&bytes)?);
        }
        Ok(())
    }

    // Byte mutation. We are looking to break the decoder. In all cases the
    // decoder should reject invalid data via `Err(error)` and exit gracefully. It should not hang/
    // segfault/ panic/ trip debug assertions or break in a any other fashion.
    #[test]
    #[ignore = "expensive"]
    fn mutate_2() -> io::Result<()> {
        let mut buddy = Buddy::default();
        let mut bytes = Vec::default();
        for seed in 0..0x0100 {
            bytes.clear();
            Seq::new(Rng::new(seed)).take(0x0100).for_each(|u| bytes.push(u));
            assert!(buddy.check_encode_decode(&bytes)?);
            for index in 0..buddy.enc.len() {
                for byte in 0..=0xFF {
                    buddy.enc[index] ^= byte;
                    let _ = buddy.decode();
                    buddy.enc[index] ^= byte;
                }
            }
            assert!(buddy.check_encode_decode(&bytes)?);
        }
        Ok(())
    }
}
