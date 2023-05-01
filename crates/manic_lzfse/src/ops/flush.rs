use super::flush_limit::FlushLimit;

pub trait Flush: FlushLimit {
    fn flush(&mut self, hard: bool) -> crate::Result<()>;
}

impl Flush for Vec<u8> {
    #[inline(always)]
    fn flush(&mut self, _: bool) -> crate::Result<()> {
        if self.len() > i32::MAX as usize {
            Err(crate::Error::BufferOverflow)
        } else {
            Ok(())
        }
    }
}

impl<T: Flush + ?Sized> Flush for &mut T {
    #[inline(always)]
    fn flush(&mut self, hard: bool) -> crate::Result<()> {
        (**self).flush(hard)
    }
}
