use spake2::{Ed25519Group, Identity, Password};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;

const MAGIC_BYTES: &[u8; 4] = b"croc";

#[derive(Debug)]
pub struct Comm {
    comm: TcpStream,
}

impl Comm {
    fn read(&mut self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut header = [0; 4];
        self.comm.read(&mut header)?;
        if &header != MAGIC_BYTES {
            Err(std::io::Error::new(ErrorKind::Other, "Magic is wrong").into())
        }
        header = [0; 4];
        self.comm.read(&mut header)?;
        let data_size: u32 = bincode::deserialize(&header)?;
        let mut buf: [u8] = (0..data_size).into_iter().map(|_| 0).collect();
        self.comm.read(&mut buf)?;
        Ok(buf.to_vec())
    }
    fn write(&mut self, buf: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut header = MAGIC_BYTES.clone();
        self.comm.write(&header)?;
        let data_size = buf.len() as u32;
        self.comm.write(&bincode::serialize(&data_size)?)?;
        OK(())
    }
}

fn main() {
    println!("Hello, world!");
}

const WEAK_KEY: [u8; 3] = [1, 2, 3];
fn init_curve(s: TcpStream) {
    let (s, key) = spake2::Spake2::<Ed25519Group>::start_a(
        &Password(WEAK_KEY.to_vec()),
        &Identity(b"server".to_vec()),
        &Identity(b"client".to_vec()),
    );
    let (s1, key1) = spake2::Spake2::
}
