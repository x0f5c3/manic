use crate::bits::{BitDst, BitWriter};
use crate::lmd::{LiteralLen, MatchDistancePack, MatchLen};

use super::constants::{self, *};
use super::error_kind::FseErrorKind;
use super::weights::Weights;
use super::Fse;

use std::convert::{From, TryFrom};
use std::fmt::{self, Debug, Formatter};

pub struct Encoder(
    [EEntry; L_SYMBOLS as usize],
    [EEntry; M_SYMBOLS as usize],
    [EEntry; D_SYMBOLS as usize],
    [EEntry; U_SYMBOLS as usize],
);

impl Encoder {
    #[inline(always)]
    pub fn init(&mut self, weights: &Weights) {
        build_e_table(weights.ls(), L_STATES, &mut self.0);
        build_e_table(weights.ms(), M_STATES, &mut self.1);
        build_e_table(weights.ds(), D_STATES, &mut self.2);
        build_e_table(weights.us(), U_STATES, &mut self.3);
    }

    /// # Safety
    ///
    /// `writer` can push `MAX_L_BITS`
    #[inline(always)]
    pub fn l<T>(&self, writer: &mut BitWriter<T>, state: &mut L, v: LiteralLen<Fse>)
    where
        T: BitDst,
    {
        debug_assert!(L_STATES <= state.0);
        debug_assert!(state.0 < 2 * L_STATES);
        let v = v.get() as usize;
        let symbol = *L_BASE_FROM_VALUE.get(v) as usize;
        debug_assert!(symbol <= L_EXTRA_BITS.len());
        let n_bits = *L_EXTRA_BITS.get(symbol) as usize;
        debug_assert!(symbol <= L_BASE_VALUE.len());
        let base_v = *L_BASE_VALUE.get(symbol) as usize;
        let bits = v as usize - base_v;
        writer.push(bits, n_bits);
        debug_assert!(symbol <= self.0.len());
        self.0.get(symbol).unwrap().encode(writer, &mut state.0)
    }

    /// # Safety
    ///
    /// `writer` can push `MAX_M_BITS`
    #[inline(always)]
    pub fn m<T>(&self, writer: &mut BitWriter<T>, state: &mut M, v: MatchLen<Fse>)
    where
        T: BitDst,
    {
        debug_assert!(M_STATES <= state.0);
        debug_assert!(state.0 < 2 * M_STATES);
        let v = v.get() as usize;
        let symbol = *M_BASE_FROM_VALUE.get(v) as usize;
        debug_assert!(symbol <= M_EXTRA_BITS.len());
        let n_bits = *M_EXTRA_BITS.get(symbol) as usize;
        debug_assert!(symbol <= M_BASE_VALUE.len());
        let base_v = *M_BASE_VALUE.get(symbol) as usize;
        let bits = v as usize - base_v;
        writer.push(bits, n_bits);
        debug_assert!(symbol <= self.1.len());
        self.1.get(symbol).unwrap().encode(writer, &mut state.0)
    }

    /// # Safety
    ///
    /// `writer` can push `MAX_D_BITS`
    #[inline(always)]
    pub fn d<T>(&self, writer: &mut BitWriter<T>, state: &mut D, v: MatchDistancePack<Fse>)
    where
        T: BitDst,
    {
        debug_assert!(D_STATES <= state.0);
        debug_assert!(state.0 < 2 * D_STATES);
        let v = v.get() as usize;
        let index = constants::d_index(v);
        debug_assert!(index <= D_BASE_FROM_VALUE.len());
        let symbol = *D_BASE_FROM_VALUE.get(index) as usize;
        debug_assert!(symbol <= D_EXTRA_BITS.len());
        let n_bits = *D_EXTRA_BITS.get(symbol) as usize;
        debug_assert!(symbol <= D_BASE_VALUE.len());
        let base_v = *D_BASE_VALUE.get(symbol) as usize;
        let bits = v - base_v;
        writer.push(bits, n_bits);
        debug_assert!(symbol <= self.2.len());
        self.2.get(symbol).unwrap().encode(writer, &mut state.0)
    }

    /// # Safety
    ///
    /// `writer` can push `MAX_U_BITS`
    #[inline(always)]
    pub fn u<T>(&self, writer: &mut BitWriter<T>, state: &mut U, u: u8)
    where
        T: BitDst,
    {
        debug_assert!(U_STATES <= state.0);
        debug_assert!(state.0 < 2 * U_STATES);
        self.3.get(u as usize).unwrap().encode(writer, &mut state.0)
    }
}

impl Debug for Encoder {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), fmt::Error> {
        f.debug_tuple("Encoder")
            .field(&self.0.as_ref())
            .field(&self.1.as_ref())
            .field(&self.2.as_ref())
            .field(&self.3.as_ref())
            .finish()
    }
}

impl Default for Encoder {
    fn default() -> Self {
        Self(
            [EEntry::default(); L_SYMBOLS as usize],
            [EEntry::default(); M_SYMBOLS as usize],
            [EEntry::default(); D_SYMBOLS as usize],
            [EEntry::default(); U_SYMBOLS as usize],
        )
    }
}

macro_rules! create_state_struct {
    ($name:ident, $max:expr, $err:expr) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        #[repr(C)]
        pub struct $name(u32);

        impl $name {
            #[inline]
            pub fn new(v: u32) -> Self {
                debug_assert!(v < $max);
                Self($max + v)
            }
        }

        impl TryFrom<u32> for $name {
            type Error = crate::Error;

            #[inline(always)]
            fn try_from(v: u32) -> Result<Self, Self::Error> {
                if v < $max {
                    Ok(Self($max + v))
                } else {
                    Err($err)
                }
            }
        }

        impl From<$name> for u32 {
            #[inline(always)]
            fn from(t: $name) -> u32 {
                debug_assert!($max <= t.0);
                debug_assert!(t.0 < 2 * $max);
                t.0 - $max
            }
        }

        impl Default for $name {
            #[inline(always)]
            fn default() -> Self {
                Self($max)
            }
        }
    };
}

create_state_struct!(L, L_STATES, FseErrorKind::BadLmdState.into());
create_state_struct!(M, M_STATES, FseErrorKind::BadLmdState.into());
create_state_struct!(D, D_STATES, FseErrorKind::BadLmdState.into());
create_state_struct!(U, U_STATES, FseErrorKind::BadLiteralState.into());

#[derive(Copy, Clone, Debug)]
#[repr(align(4))]
pub struct EEntry {
    t_k: i16,
    t_w: i16,
}

impl Default for EEntry {
    #[inline(always)]
    fn default() -> Self {
        Self { t_k: 0, t_w: 0 }
    }
}

impl EEntry {
    #[inline(always)]
    pub fn encode<T: BitDst>(self, writer: &mut BitWriter<T>, state: &mut u32) {
        let s = *state;
        let n_bits = (self.t_k as i32 + s as i32) as u32 >> 10;
        *state = (self.t_w as i32 + ((s as i32) >> n_bits)) as u32;
        debug_assert!(n_bits <= 10);
        let mask = *MASK_TABLE.get(n_bits as usize).unwrap() as usize;
        let bits = s as usize & mask;
        writer.push(bits, n_bits as usize);
    }
}

const MASK_TABLE: [u32; 11] = [
    0x0000_0000,
    0x0000_0001,
    0x0000_0003,
    0x0000_0007,
    0x0000_000F,
    0x0000_001F,
    0x0000_003F,
    0x0000_007F,
    0x0000_00FF,
    0x0000_01FF,
    0x0000_03FF,
];

#[allow(arithmetic_overflow)]
#[allow(clippy::needless_range_loop)]
#[inline(always)]
pub fn build_e_table(weights: &[u16], n_states: u32, table: &mut [EEntry]) {
    assert_eq!(weights.len(), table.len());
    assert!(n_states.is_power_of_two());
    assert!(n_states <= 1024);
    let n_clz = n_states.leading_zeros();
    let mut e = EEntry::default();
    let mut total = 0;
    for i in 0..weights.len() {
        let w = *{ weights.get(i) } as u32;
        if w == 0 {
            e.t_k = -(n_states as i16);
            e.t_w = 0;
        } else {
            debug_assert!(total + w <= n_states);
            let k = w.leading_zeros() - n_clz;
            e.t_k = 1024 * k as i16 - ((w as u32) << k) as i16;
            e.t_w = n_states as i16 + total as i16 - w as i16;
        }
        *{ table.get_mut(i) } = e;
        total += w;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use std::io;

    #[test]
    fn test_null() -> io::Result<()> {
        let mut weights = [4; 256];
        weights[0] = 0;
        let mut table = [EEntry::default(); 256];
        build_e_table(&weights, U_STATES, &mut table);
        let mut dst = Vec::default();
        let mut wtr = BitWriter::new(&mut dst, 0)?;
        let mut state = U::default();
        for _ in 0..32 {
            table[0].encode(&mut wtr, &mut state.0);
        }
        let n = wtr.finalize()?;
        assert_eq!(n, 0);
        assert_eq!(dst.len(), 0);
        Ok(())
    }
}
