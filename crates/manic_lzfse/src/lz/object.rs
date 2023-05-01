use crate::kit::{Width, W08, W16};

use std::hint::unreachable_unchecked;
use std::ptr;

// 8 byte buffer expansion look up tables.
// Indices 0, 1 are undefined and should not be used.
const OFFSET: [u32; 8] = [0, 0, 2, 1, 0, 4, 4, 4];
const DELTA: [u32; 8] = [0, 0, 2, 2, 4, 3, 2, 1];

// Implementation notes:
//
// Public function method signatures are low level benchmark driven as opposed to a more
// conventional: `(dst: *mut u8, len: usize, distance: usize)`.

/// 16 byte unaligned copy.
///
/// `len = dst_end.sub(dst)`
/// `distance = dst.sub(src)`
///
/// # Safety
///
/// * `src` is valid for `len + 16` byte reads.
/// * `dst` is valid for `len + 16` byte writes.
/// * `8 <= distance`
#[inline(always)]
pub unsafe fn write_match_16(src: *const u8, dst: *mut u8, dst_end: *mut u8) {
    copy_wide_match::<W16>(src, dst, dst_end);
}

/// 16 byte register full/ partial expansion with unaligned store.
///
/// `len = dst_end.sub(dst)`
/// `distance = dst.sub(src)`
///
/// # Safety
///
/// * `src` is valid for `len + WIDE` byte reads.
/// * `dst` is valid for `len + WIDE` byte writes.
/// * `8 <= distance`
/// * `distance <= 16`
// TODO benchmark, may be inefficient on 32 bit builds.
#[inline(always)]
pub unsafe fn write_match_8(src: *const u8, dst: *mut u8, dst_end: *mut u8, distance: usize) {
    delta_16(expand_r16(src), dst, dst_end, distance)
}

/// 8 byte register full/ partial expansion with unaligned store.
///
/// `len = dst_end.sub(dst)`
/// `distance = dst.sub(src)`
///

///
/// * `src` is valid for `len + WIDE` byte reads.
/// * `dst` is valid for `len + WIDE` byte writes.
/// * `1 <=distance`
/// * `distance <= 8`
#[inline(always)]
pub unsafe fn write_match_x(src: *const u8, dst: *mut u8, dst_end: *mut u8, distance: usize) {
    debug_assert!(0 < distance);
    debug_assert!(distance <= 8);
    match distance {
        8 => store_8(expand_r8(src), dst, dst_end),
        7 => delta_8(expand_r8(src), dst, dst_end, 7),
        6 => delta_8(expand_r8(src), dst, dst_end, 6),
        5 => delta_8(expand_r8(src), dst, dst_end, 5),
        4 => store_8(expand_r4(src), dst, dst_end),
        3 => delta_8(expand_r3(src), dst, dst_end, 6),
        2 => store_8(expand_r2(src), dst, dst_end),
        1 => store_8(expand_r1(src), dst, dst_end),
        _ => unreachable_unchecked(),
    };
}

/// 8 byte memory expansion with unaligned copy.
///
/// `len = dst_end.sub(dst)`
/// `distance = dst.sub(src)`
///

///
/// * `src` is valid for `len + WIDE` byte reads.
/// * `dst` is valid for `len + WIDE` byte writes.
/// * `1 <=distance`
/// * `distance <= 16`
#[allow(clippy::identity_op)]
#[allow(dead_code)]
#[inline(always)]
pub unsafe fn write_match_alt(src: *const u8, dst: *mut u8, dst_end: *mut u8, distance: usize) {
    debug_assert!(0 < distance);
    debug_assert!(distance <= 16);
    let delta = if distance < 8 {
        let u = *src.add(0);
        let v = *src.add(1);
        *dst.add(0) = u;
        *dst.add(1) = v;
        let u = *src.add(2);
        let v = *src.add(3);
        *dst.add(2) = u;
        *dst.add(3) = v;
        ptr::copy_nonoverlapping(src.add(OFFSET[distance] as usize), dst.add(4), 4);
        DELTA[distance as usize] as usize
    } else {
        ptr::copy_nonoverlapping(src, dst, 8);
        8
    };
    let dst = dst.add(8);
    if dst >= dst_end {
    } else {
        let src = src.add(delta);
        copy_wide_match::<W08>(src, dst, dst_end);
    }
}

#[inline(always)]
unsafe fn expand_r1(src: *const u8) -> [u8; 8] {
    (src.cast::<u8>().read_unaligned() as u64 * 0x0101_0101_0101_0101).to_ne_bytes()
}

#[inline(always)]
unsafe fn expand_r2(src: *const u8) -> [u8; 8] {
    (src.cast::<u16>().read_unaligned() as u64 * 0x0001_0001_0001_0001).to_ne_bytes()
}

#[inline(always)]
#[cfg(target_endian = "little")]
unsafe fn expand_r3(src: *const u8) -> [u8; 8] {
    ((src.cast::<u32>().read_unaligned() & 0x00FF_FFFF) as u64 * 0x0000_0000_0100_0001)
        .to_ne_bytes()
}

#[inline(always)]
#[cfg(target_endian = "big")]
unsafe fn expand_r3(src: *const u8) -> [u8; 8] {
    ((src.cast::<u32>().read_unaligned() & 0xFFFF_FF00) as u64 * 0x0000_0001_0000_0100)
        .to_ne_bytes()
}

#[inline(always)]
unsafe fn expand_r4(src: *const u8) -> [u8; 8] {
    (src.cast::<u32>().read_unaligned() as u64 * 0x0000_0001_0000_0001).to_ne_bytes()
}

#[inline(always)]
unsafe fn expand_r8(src: *const u8) -> [u8; 8] {
    src.cast::<u64>().read_unaligned().to_ne_bytes()
}

#[inline(always)]
unsafe fn expand_r16(src: *const u8) -> [u8; 16] {
    src.cast::<u128>().read_unaligned().to_ne_bytes()
}

unsafe fn store_8(reg: [u8; 8], mut dst: *mut u8, dst_end: *mut u8) {
    let src = reg.as_ptr();
    loop {
        ptr::copy_nonoverlapping(src, dst, 8);
        dst = dst.add(8);
        if dst >= dst_end {
            break;
        }
    }
}

unsafe fn delta_8(reg: [u8; 8], mut dst: *mut u8, dst_end: *mut u8, delta: usize) {
    debug_assert!(delta < 8);
    let src = reg.as_ptr();
    loop {
        ptr::copy_nonoverlapping(src, dst, 8);
        dst = dst.add(delta);
        if dst >= dst_end {
            break;
        }
    }
}

unsafe fn delta_16(reg: [u8; 16], mut dst: *mut u8, dst_end: *mut u8, delta: usize) {
    debug_assert!(delta <= 16);
    let src = reg.as_ptr();
    loop {
        ptr::copy_nonoverlapping(src, dst, 16);
        dst = dst.add(delta);
        if dst >= dst_end {
            break;
        }
    }
}

unsafe fn copy_wide_match<W: Width>(mut src: *const u8, mut dst: *mut u8, dst_end: *mut u8) {
    ptr::copy_nonoverlapping(src, dst, W::WIDTH);
    dst = dst.add(W::WIDTH);
    src = src.add(W::WIDTH);
    if dst >= dst_end {
        return;
    }
    copy_wide_match_cont::<W>(src, dst, dst_end);
}

unsafe fn copy_wide_match_cont<W: Width>(mut src: *const u8, mut dst: *mut u8, dst_end: *mut u8) {
    loop {
        ptr::copy_nonoverlapping(src, dst, W::WIDTH);
        dst = dst.add(W::WIDTH);
        src = src.add(W::WIDTH);
        if dst >= dst_end {
            break;
        }
    }
}
