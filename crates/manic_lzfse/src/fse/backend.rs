use crate::encode::Backend;
use crate::fse::{Buffer, Encoder, Weights};
use crate::lmd::MatchDistance;
use crate::types::{ShortBuffer, ShortWriter};

use super::constants::*;
use super::object::Fse;

use std::io;

pub struct FseBackend {
    buffer: Buffer,
    weights: Weights,
    encoder: Encoder,
}

impl FseBackend {
    #[allow(dead_code)]
    #[cold]
    fn emit_block_v1<O: ShortWriter>(&mut self, dst: &mut O, flush: bool) -> io::Result<()> {
        let mark = dst.pos();
        dst.write_short_bytes(&[0u8; V1_HEADER_SIZE as usize])?;
        self.buffer.pad();
        self.buffer.init_weights(&mut self.weights);
        self.weights.store_v1_short(dst)?;
        self.encoder.init(&self.weights);
        let block = self.buffer.store(dst, &self.encoder)?;
        let bytes = dst.patch_into(mark, V1_HEADER_SIZE as usize);
        block.store_v1(bytes);
        self.buffer.reset();
        if flush {
            dst.flush(false)?;
        }
        Ok(())
    }

    #[cold]
    fn emit_block_v2<O: ShortWriter>(&mut self, dst: &mut O, flush: bool) -> io::Result<()> {
        let mark = dst.pos();
        dst.write_short_bytes(&[0u8; V2_HEADER_SIZE as usize])?;
        self.buffer.pad();
        self.buffer.init_weights(&mut self.weights);
        let n_weight_payload_bytes = self.weights.store_v2_short(dst)?;
        self.encoder.init(&self.weights);
        let block = self.buffer.store(dst, &self.encoder)?;
        let bytes = dst.patch_into(mark, V2_HEADER_SIZE as usize);
        block.store_v2(bytes, n_weight_payload_bytes);
        self.buffer.reset();
        if flush {
            dst.flush(false)?;
        }
        Ok(())
    }
}

impl Backend for FseBackend {
    type Type = Fse;

    #[inline(always)]
    fn init<O: ShortWriter>(&mut self, _: &mut O, _: Option<u32>) -> io::Result<()> {
        self.buffer.reset();
        Ok(())
    }

    #[inline(always)]
    fn push_literals<I: ShortBuffer, O: ShortWriter>(
        &mut self,
        dst: &mut O,
        literals: I,
    ) -> io::Result<()> {
        self.push_match(dst, literals, 0, unsafe { MatchDistance::new(1) })
    }

    #[inline(always)]
    fn push_match<I: ShortBuffer, O: ShortWriter>(
        &mut self,
        dst: &mut O,
        mut literals: I,
        mut match_len: u32,
        match_distance: MatchDistance<Fse>,
    ) -> io::Result<()> {
        loop {
            if self.buffer.push(&mut literals, &mut match_len, match_distance) {
                break;
            }
            self.emit_block_v2(dst, true)?;
        }
        Ok(())
    }

    fn finalize<O: ShortWriter>(&mut self, dst: &mut O) -> io::Result<()> {
        self.emit_block_v2(dst, false)?;
        Ok(())
    }
}

impl Default for FseBackend {
    fn default() -> Self {
        Self { buffer: Buffer::default(), weights: Weights::default(), encoder: Encoder::default() }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::base::MagicBytes;
//     use crate::ops::WriteShort;

//     use std::fs;

//     use super::*;

//     // Single block tiny high compression ratio file.
//     #[test]
//     fn explode_tiny() -> io::Result<()> {
//         let mut block = Vec::default();
//         let mut backend = FseBackend::default();
//         backend.init(&mut block, None)?;
//         for _ in 0..LMDS_PER_BLOCK {
//             backend.push_match(&mut block, [0; 4].as_ref(), 15, MatchDistance::new(1))?;
//         }
//         backend.finalize(&mut block)?;
//         block.write_short_u32(MagicBytes::Eos.into())?;
//         fs::write("data/special/explode_tiny", block)?;
//         Ok(())
//     }

//     // Single block medium high compression ratio file.
//     #[test]
//     fn explode_med() -> io::Result<()> {
//         let mut block = Vec::default();
//         let mut backend = FseBackend::default();
//         backend.init(&mut block, None)?;
//         for _ in 0..LMDS_PER_BLOCK {
//             backend.push_match(
//                 &mut block,
//                 [0; 4].as_ref(),
//                 MAX_M_VALUE as u32,
//                 MatchDistance::new(1),
//             )?;
//         }
//         backend.finalize(&mut block)?;
//         block.write_short_u32(MagicBytes::Eos.into())?;
//         fs::write("data/special/explode_medium.lzfse", block)?;
//         Ok(())
//     }
// }
