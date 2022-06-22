#![allow(dead_code)]
use educe::Educe;
use secrecy::{SecretString, SecretVec};
use tokio::net::TcpStream;
use zeroize::Zeroize;

#[derive(Educe)]
#[educe(Debug)]
pub struct Relay {
    host: String,
    port: String,
    banner: String,
    password: SecretString,
    #[educe(Debug(ignore))]
    rooms: SecretVec<Room>,
}

#[derive(Debug, Zeroize)]
pub struct Room {
    pin: String,
    #[zeroize(skip)]
    sender: Vec<TcpStream>,
    #[zeroize(skip)]
    receiver: Vec<TcpStream>,
    secured: bool,
    #[zeroize(skip)]
    opened: std::time::Instant,
}
