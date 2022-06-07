pub mod codec;
pub mod error;

pub use codec::{Codec, Reader, Writer};
pub use error::CodecError;
pub use manic_proto::Packet;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
