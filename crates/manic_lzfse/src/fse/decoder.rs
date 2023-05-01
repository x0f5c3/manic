use crate::bits::{BitReader, BitSrc};
use crate::lmd::{LiteralLen, MatchDistancePack, MatchLen};

use super::constants::*;
use super::error_kind::FseErrorKind;
use super::object::Fse;
use super::weights::Weights;

use std::convert::{From, TryFrom};
use std::fmt::{self, Debug, Formatter};

pub struct Decoder(
    [VEntry; L_STATES as usize],
    [VEntry; M_STATES as usize],
    [VEntry; D_STATES as usize],
    [UEntry; U_STATES as usize],
);

impl Decoder {
    pub fn init(&mut self, weights: &Weights) {
        build_v_table(weights.ls(), &L_EXTRA_BITS, &L_BASE_VALUE, &mut self.0);
        build_v_table(weights.ms(), &M_EXTRA_BITS, &M_BASE_VALUE, &mut self.1);
        build_v_table(weights.ds(), &D_EXTRA_BITS, &D_BASE_VALUE, &mut self.2);
        build_u_table(weights.us(), &mut self.3);
    }

    /// # Safety
    ///
    /// `reader` can pull `MAX_L_BITS`
    #[inline(always)]
    pub unsafe fn l<T>(&self, reader: &mut BitReader<T>, state: &mut L) -> LiteralLen<Fse>
    where
        T: BitSrc,
    {
        debug_assert!(state.0 < L_STATES as usize);
        LiteralLen::new(self.0.get(state.0).decode(reader, &mut state.0))
    }

    /// # Safety
    ///
    /// `reader` can pull `MAX_M_BITS`
    #[inline(always)]
    pub unsafe fn m<T>(&self, reader: &mut BitReader<T>, state: &mut M) -> MatchLen<Fse>
    where
        T: BitSrc,
    {
        debug_assert!(state.0 < M_STATES as usize);
        MatchLen::new(self.1.get(state.0).decode(reader, &mut state.0))
    }

    /// # Safety
    ///
    /// `reader` can pull `MAX_D_BITS`
    #[inline(always)]
    pub unsafe fn d<T: BitSrc>(
        &self,
        reader: &mut BitReader<T>,
        state: &mut D,
    ) -> MatchDistancePack<Fse> {
        debug_assert!(state.0 < D_STATES as usize);
        MatchDistancePack::new(self.2.get(state.0).decode(reader, &mut state.0))
    }

    /// # Safety
    ///
    /// `reader` can pull `MAX_U_BITS`
    #[inline(always)]
    pub unsafe fn u<T>(&self, reader: &mut BitReader<T>, state: &mut U) -> u8
    where
        T: BitSrc,
    {
        debug_assert!(state.0 < U_STATES as usize);
        self.3.get(state.0).decode(reader, &mut state.0)
    }
}

impl Debug for Decoder {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), fmt::Error> {
        f.debug_tuple("Decoder")
            .field(&self.0.as_ref())
            .field(&self.1.as_ref())
            .field(&self.2.as_ref())
            .field(&self.3.as_ref())
            .finish()
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self(
            [VEntry::default(); L_STATES as usize],
            [VEntry::default(); M_STATES as usize],
            [VEntry::default(); D_STATES as usize],
            [UEntry::default(); U_STATES as usize],
        )
    }
}

macro_rules! create_state_struct {
    ($name:ident, $max:expr, $err:expr) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        pub struct $name(usize);

        impl $name {
            #[inline]
            pub fn new(v: usize) -> Self {
                debug_assert!(v < $max);
                Self(v)
            }
        }

        impl TryFrom<usize> for $name {
            type Error = crate::Error;

            #[inline(always)]
            fn try_from(v: usize) -> Result<Self, Self::Error> {
                if v < $max {
                    Ok(Self(v))
                } else {
                    Err($err)
                }
            }
        }

        impl From<$name> for usize {
            #[inline(always)]
            fn from(t: $name) -> usize {
                debug_assert!(t.0 < $max);
                t.0
            }
        }

        impl Default for $name {
            #[inline(always)]
            fn default() -> Self {
                Self(0)
            }
        }
    };
}

create_state_struct!(L, L_STATES as usize, FseErrorKind::BadLmdState.into());
create_state_struct!(M, M_STATES as usize, FseErrorKind::BadLmdState.into());
create_state_struct!(D, D_STATES as usize, FseErrorKind::BadLmdState.into());
create_state_struct!(U, U_STATES as usize, FseErrorKind::BadLiteralState.into());

#[derive(Copy, Clone, Debug, Default)]
#[repr(align(8))]
pub struct VEntry {
    k: u8,
    v_bits: u8,
    delta: i16,
    v_base: u32,
}

impl VEntry {
    #[inline(always)]
    fn decode<T: BitSrc>(self, bsi: &mut BitReader<T>, state: &mut usize) -> u32 {
        *state = (bsi.pull(self.k as usize) as isize + self.delta as isize) as usize;
        self.v_base + bsi.pull(self.v_bits as usize) as u32
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(align(4))]
pub struct UEntry {
    k: u8,
    symbol: u8,
    delta: i16,
}

impl UEntry {
    #[inline(always)]
    pub unsafe fn decode<T: BitSrc>(self, reader: &mut BitReader<T>, state: &mut usize) -> u8 {
        *state = (reader.pull(self.k as usize) as isize + self.delta as isize) as usize;
        self.symbol
    }
}

impl Default for UEntry {
    #[inline(always)]
    fn default() -> Self {
        Self { k: 0, symbol: 0, delta: 0 }
    }
}

#[allow(arithmetic_overflow)]
#[allow(clippy::needless_range_loop)]
pub fn build_v_table(
    weights: &[u16],
    v_bits_table: &[u8],
    v_base_table: &[u32],
    table: &mut [VEntry],
) {
    assert_eq!(v_bits_table.len(), weights.len());
    assert_eq!(v_base_table.len(), weights.len());
    let n_states = table.len() as u32;
    assert!(n_states.is_power_of_two());
    let n_clz = n_states.leading_zeros();
    let mut e = VEntry::default();
    let mut total = 0;
    for i in 0..weights.len() {
        let w = *{ weights.get(i).unwrap() } as u32;
        if w == 0 {
            continue;
        }
        debug_assert!(total + w <= n_states);
        let k = w.leading_zeros() - n_clz;
        let x = ((n_states << 1) >> k) - w;
        let v_bits = *{ v_bits_table.get(i as usize).unwrap() };
        let v_base = *{ v_base_table.get(i as usize).unwrap() };
        e.k = k as u8;
        e.v_bits = v_bits;
        e.v_base = v_base;
        for j in 0..x {
            e.delta = (((w as i32 + j as i32) << k) - n_states as i32) as i16;
            *{ table.get_mut((total + j as u32) as usize).unwrap() } = e;
        }
        e.k = (k as i32 - 1) as u8;
        for j in x..w {
            e.delta = ((j - x) << (k - 1)) as i16;
            *{ table.get_mut((total + j as u32) as usize).unwrap() } = e;
        }
        total += w;
    }
    for i in (total as usize)..table.len() {
        *{ table.get_mut(i).unwrap() } = VEntry::default();
    }
}

#[allow(arithmetic_overflow)]
#[allow(clippy::needless_range_loop)]
pub fn build_u_table(weights: &[u16], table: &mut [UEntry]) {
    let n_states = table.len() as u32;
    assert!(n_states.is_power_of_two());
    let n_clz = n_states.leading_zeros();
    let mut e = UEntry::default();
    let mut total = 0;
    for i in 0..weights.len() {
        let w = *{ weights.get(i).unwrap() } as u32;
        if w == 0 {
            continue;
        }
        debug_assert!(total + w <= n_states);
        let k = w.leading_zeros() - n_clz;
        let x = ((n_states << 1) >> k) - w;
        e.symbol = i as u8;
        e.k = k as u8;
        for j in 0..x {
            e.delta = (((w as i32 + j as i32) << k) - n_states as i32) as i16;
            *{ table.get_mut((total + j as u32) as usize).unwrap() } = e;
        }
        e.k = (k as i32 - 1) as u8;
        for j in x..w {
            e.delta = ((j - x) << (k - 1)) as i16;
            *{ table.get_mut((total + j as u32) as usize).unwrap() } = e;
        }
        total += w;
    }
    for i in (total as usize)..table.len() {
        *{ table.get_mut(i).unwrap() } = UEntry::default();
    }
}
