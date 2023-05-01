use crate::ops::Len;

/// BitReader source. Lazy underflow state management. 8 byte padded (undefined).
pub trait BitSrc: Len {
    /// Pops bytes, as little-endian `usize` packed to the right with any unused bytes undefined.
    /// Usage before initialization undefined not unsafe.
    ///
    /// `n_bytes < size_of::<usize>()`
    unsafe fn pop_bytes(&mut self, n_bytes: usize) -> usize;

    /// Initialize and pop `size_of::<usize> - 1` bytes with unused bytes set to zero.
    fn init_1(&mut self) -> usize;

    /// Initialize and pop `size_of::<usize>` bytes.
    fn init_0(&mut self) -> usize;
}
