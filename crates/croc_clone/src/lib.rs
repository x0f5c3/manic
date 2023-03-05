mod cli;
mod comm;
mod compress;
mod croc;
mod crypt;
mod install;
mod message;
mod models;
mod tcp;
mod utils;
mod error;


pub use error::{Result, CodecError, SpakeError, PWHashError};

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
