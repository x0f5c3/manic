use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2,
};
use spake2::{Ed25519Group, Identity, Password};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;

const MAGIC_BYTES: &[u8; 4] = b"croc";

#[derive(Debug)]
pub struct Comm {
    comm: TcpStream,
}

impl Comm {
    fn read(&mut self) -> Result<Vec<u8>> {
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
    fn write(&mut self, buf: &[u8]) -> Result<()> {
        let mut header = MAGIC_BYTES.clone();
        self.comm.write(&header)?;
        let data_size = buf.len() as u32;
        self.comm.write(&bincode::serialize(&data_size)?)?;
        Ok(())
    }
}

fn main() {
    println!("Hello, world!");
}

enum SpakeSide {
    Server,
    Client,
}

const WEAK_KEY: [u8; 3] = [1, 2, 3];
fn init_curve(mut st: Comm) {
    let (s, key) = spake2::Spake2::<Ed25519Group>::start_a(
        &Password(WEAK_KEY.to_vec()),
        &Identity(b"server".to_vec()),
        &Identity(b"client".to_vec()),
    );
    st.write(&key).unwrap();
    let bbytes = st.read().unwrap();
    let strong_key = s.finish(&bbytes).unwrap();
    let pw_hash = new_argon2(&strong_key).unwrap();
    st.write(pw_hash.salt.unwrap().as_bytes()).unwrap();
}

fn new_argon2(pw: &[u8]) -> Result<PasswordHash> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let pw_hash = argon2.hash_password(pw, &salt)?;
    Ok(pw_hash)
}
