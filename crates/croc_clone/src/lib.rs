mod cli;
mod comm;
mod compress;
mod croc;
mod crypt;
mod error;
mod install;
mod message;
mod models;
mod options;
mod tcp;
mod utils;
mod xxwriter;

pub use error::{CrocError, PWHashError, Result, SpakeError};
pub use message::*;
pub const MAGIC_BYTES: &[u8] = b"manic";

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
