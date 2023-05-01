pub trait FlushLimit {
    /// Max number of bytes accessed between flush calls.
    const FLUSH_LIMIT: u32;
}

impl FlushLimit for Vec<u8> {
    const FLUSH_LIMIT: u32 = i32::MAX as u32;
}

impl<T: FlushLimit + ?Sized> FlushLimit for &mut T {
    const FLUSH_LIMIT: u32 = T::FLUSH_LIMIT;
}
