use crate::ops::ShortLimit;

use std::marker::PhantomData;
use std::mem;

// Implementation notes:
//
// To limit the propagation of unsafe code and differentiate between similar data structures we
// call upon unit types.
//
// Literal/ match lengths can be packed into u16 structures.
//
// Match distances cannot be zero, however packed/ zeroed match distances are employed by FSE
// blocks to improve compression. Here given a sequence of identical match distances, the first is
// encoded as is and the remaining are encoded as zero. Decoding is simply the reverse of this.
// As a caveat a badly formed packed sequence may yield zero match distance values which we must
// check for. Thus we end up with an asymmetrical encode/ decode pattern:
// MatchDistance -> MatchDistancePack -> encode/ decode -> MatchDistancePack -> MatchDistanceUnpack
//
// Although identical in terms of internal limits, a zero length MatchDistancePack is normal whilst
// a zero distance MatchDistanceUnpack is an error state.

pub trait LmdMax: LMax + MMax + DMax + Copy + Clone {}

pub trait LMax: Copy {
    const MAX_LITERAL_LEN: u16;
}

pub trait MMax: Copy {
    const MAX_MATCH_LEN: u16;
}

pub trait DMax: Copy {
    const MAX_MATCH_DISTANCE: u32;
}

#[derive(Copy, Clone)]
pub struct Quad;

impl LMax for Quad {
    const MAX_LITERAL_LEN: u16 = mem::size_of::<u32>() as u16;
}

macro_rules! create_type_struct {
    ($name:ident, $u:ty, $t:ident, $min:expr, $max:ident) => {
        #[derive(Copy, Clone, Debug, Eq, PartialEq)]
        pub struct $name<T>($u, PhantomData<T>);

        impl<T: $t> $name<T> {
            #[inline(always)]
            #[allow(dead_code)]
            #[allow(unused_comparisons)]
            pub fn new(u: $u) -> Self {
                assert!($min <= u);
                assert!(u <= T::$max as $u);
                Self(u, PhantomData::default())
            }

            #[allow(unused_comparisons)]
            #[inline(always)]
            pub unsafe fn new_unchecked(u: $u) -> Self {
                debug_assert!($min <= u);
                debug_assert!(u <= T::$max as $u);
                Self(u, PhantomData::default())
            }

            #[inline(always)]
            pub fn get(self) -> $u {
                self.0
            }
        }

        impl<T: $t> Default for $name<T> {
            #[inline(always)]
            fn default() -> Self {
                Self($min, PhantomData::default())
            }
        }
    };
}

// Bounded literal len.
// 0..=MAX_LITERAL_LEN
// u32
create_type_struct!(LiteralLen, u32, LMax, 0, MAX_LITERAL_LEN);

impl<T: LMax> From<LiteralLenPack<T>> for LiteralLen<T> {
    #[inline(always)]
    fn from(other: LiteralLenPack<T>) -> Self {
        unsafe { Self::new(other.0 as u32) }
    }
}

impl<T: LMax> ShortLimit for LiteralLen<T> {
    const SHORT_LIMIT: u32 = T::MAX_LITERAL_LEN as u32;
}

// Bounded packed literal len.
// 0..=MAX_LITERAL_LEN
// u16
create_type_struct!(LiteralLenPack, u16, LMax, 0, MAX_LITERAL_LEN);

impl<T: LMax> From<LiteralLen<T>> for LiteralLenPack<T> {
    #[inline(always)]
    fn from(other: LiteralLen<T>) -> Self {
        unsafe { Self::new(other.0 as u16) }
    }
}

// Bounded match len.
// 0..=MAX_MATCH_LEN
// u32
create_type_struct!(MatchLen, u32, MMax, 0, MAX_MATCH_LEN);

impl<T: MMax> From<MatchLenPack<T>> for MatchLen<T> {
    #[inline(always)]
    fn from(other: MatchLenPack<T>) -> Self {
        unsafe { Self::new(other.0 as u32) }
    }
}

impl<T: MMax> ShortLimit for MatchLen<T> {
    const SHORT_LIMIT: u32 = T::MAX_MATCH_LEN as u32;
}

// Bounded packed zeroed match length.
// 0..=MAX_MATCH_LEN
// u16
create_type_struct!(MatchLenPack, u16, MMax, 0, MAX_MATCH_LEN);

impl<T: MMax> From<MatchLen<T>> for MatchLenPack<T> {
    #[inline(always)]
    fn from(other: MatchLen<T>) -> Self {
        unsafe { Self::new(other.0 as u16) }
    }
}

// Bounded nonzero match distance.
// 1..=MAX_MATCH_DISTANCE
// u32
create_type_struct!(MatchDistance, u32, DMax, 1, MAX_MATCH_DISTANCE);

// Bounded packed match distance.
// 0..=MAX_MATCH_DISTANCE
// u32
create_type_struct!(MatchDistancePack, u32, DMax, 0, MAX_MATCH_DISTANCE);

// Bounded unpacked match distance.
// 0..=MAX_MATCH_DISTANCE
// u32
create_type_struct!(MatchDistanceUnpack, u32, DMax, 0, MAX_MATCH_DISTANCE);

impl<T: DMax> MatchDistanceUnpack<T> {
    #[inline(always)]
    pub fn substitute(&mut self, other: MatchDistancePack<T>) {
        if other.0 != 0 {
            self.0 = other.0
        }
    }
}

impl<T: DMax> From<MatchDistance<T>> for MatchDistanceUnpack<T> {
    #[inline(always)]
    fn from(other: MatchDistance<T>) -> Self {
        Self(other.get(), PhantomData::default())
    }
}
