use crate::ops::{Len, Limit, Skip};
use crate::Error;

pub trait Take: Copy {
    fn take(&mut self, n_payload_bytes: u32) -> crate::Result<Self>;
}

impl<T: Copy + Len + Skip + Limit> Take for T {
    #[inline(always)]
    fn take(&mut self, n_payload_bytes: u32) -> crate::Result<Self> {
        let n_payload_bytes = n_payload_bytes as usize;
        if self.len() < n_payload_bytes {
            return Err(Error::PayloadUnderflow);
        }
        let mut payload = *self;
        payload.limit(n_payload_bytes);
        self.skip(n_payload_bytes);
        Ok(payload)
    }
}
