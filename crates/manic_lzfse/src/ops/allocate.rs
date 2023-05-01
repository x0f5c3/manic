use std::io;

pub trait Allocate {
    /// Allocate `len` bytes returning `io::ErrorKind::Other` in case of failure.
    fn allocate(&mut self, len: usize) -> io::Result<()>;

    fn is_allocated(&mut self, len: usize) -> bool;
}

impl<T: Allocate + ?Sized> Allocate for &mut T {
    #[inline(always)]
    fn allocate(&mut self, len: usize) -> io::Result<()> {
        (**self).allocate(len)
    }

    #[inline(always)]
    fn is_allocated(&mut self, len: usize) -> bool {
        (**self).is_allocated(len)
    }
}

impl Allocate for Vec<u8> {
    // TODO: Issue 48043, use `vec::try_reserve`
    #[cfg(target_pointer_width = "32")]
    fn allocate(&mut self, len: usize) -> io::Result<()> {
        let index = self.len();
        if !self.is_allocated(len) {
            if index + len > isize::MAX as usize {
                // Unlikely.
                return Err(io::ErrorKind::Other.into());
            }
            self.reserve(len);
        }
        Ok(())
    }

    #[cfg(target_pointer_width = "64")]
    fn allocate(&mut self, additional: usize) -> io::Result<()> {
        self.reserve(additional);
        Ok(())
    }

    #[inline(always)]
    fn is_allocated(&mut self, len: usize) -> bool {
        len <= self.capacity().wrapping_sub(self.len())
    }
}
