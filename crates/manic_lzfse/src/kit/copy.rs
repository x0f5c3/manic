use super::wide::Width;

use std::ptr;

pub trait CopyType {
    /// # Safety
    ///
    /// * `src` is valid for `len + W::WIDTH` byte reads.
    /// * `dst` is valid for `len + W::WIDTH` byte writes.
    unsafe fn wide_copy<W: Width>(src: *const u8, dst: *mut u8, len: usize);
}

#[derive(Copy, Clone, Debug)]
pub struct CopyTypeIndex;

impl CopyType for CopyTypeIndex {
    #[inline(always)]
    unsafe fn wide_copy<W: Width>(src: *const u8, dst: *mut u8, len: usize) {
        let mut off = 0;
        loop {
            ptr::copy_nonoverlapping(src.add(off), dst.add(off), W::WIDTH);
            off += W::WIDTH;
            if off >= len {
                break;
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CopyTypePtr;

impl CopyType for CopyTypePtr {
    #[inline(always)]
    unsafe fn wide_copy<W: Width>(mut src: *const u8, mut dst: *mut u8, len: usize) {
        let dst_end = dst.add(len);
        loop {
            ptr::copy_nonoverlapping(src, dst, W::WIDTH);
            dst = dst.add(W::WIDTH);
            src = src.add(W::WIDTH);
            if dst >= dst_end {
                break;
            }
        }
    }
}

// High latency, high throughput.
#[derive(Copy, Clone, Debug)]
pub struct CopyTypeLong;

impl CopyType for CopyTypeLong {
    #[inline(always)]
    unsafe fn wide_copy<W: Width>(src: *const u8, dst: *mut u8, len: usize) {
        const K: usize = 8;
        let mut off = 0;
        if len >= W::WIDTH * K {
            let wide_len = (len / W::WIDTH / K) * W::WIDTH * K;
            loop {
                ptr::copy_nonoverlapping(src.add(off), dst.add(off), W::WIDTH * K);
                off += W::WIDTH * K;
                if off == wide_len {
                    break;
                }
            }
        }
        loop {
            ptr::copy_nonoverlapping(src.add(off), dst.add(off), W::WIDTH);
            off += W::WIDTH;
            if off >= len {
                break;
            }
        }
    }
}
