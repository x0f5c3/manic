use std::convert::{From, TryFrom};

const BM_EOS: u32 = 0x2478_7662;
const BM_RAW: u32 = 0x2D78_7662;
const BM_VX1: u32 = 0x3178_7662;
const BM_VX2: u32 = 0x3278_7662;
const BM_VXN: u32 = 0x6E78_7662;

#[derive(Copy, Clone, Debug)]
pub enum MagicBytes {
    Eos,
    Raw,
    Vx1,
    Vx2,
    Vxn,
}

impl TryFrom<u32> for MagicBytes {
    type Error = crate::Error;

    #[inline(always)]
    fn try_from(u: u32) -> Result<MagicBytes, Self::Error> {
        match u {
            BM_EOS => Ok(MagicBytes::Eos),
            BM_RAW => Ok(MagicBytes::Raw),
            BM_VX1 => Ok(MagicBytes::Vx1),
            BM_VX2 => Ok(MagicBytes::Vx2),
            BM_VXN => Ok(MagicBytes::Vxn),
            _ => Err(crate::Error::BadBlock(u)),
        }
    }
}

impl From<MagicBytes> for u32 {
    #[inline(always)]
    fn from(v: MagicBytes) -> Self {
        match v {
            MagicBytes::Eos => BM_EOS,
            MagicBytes::Raw => BM_RAW,
            MagicBytes::Vx1 => BM_VX1,
            MagicBytes::Vx2 => BM_VX2,
            MagicBytes::Vxn => BM_VXN,
        }
    }
}
