use futures::prelude::*;
use futures::{Sink, SinkExt};
use manic_proto::BincodeCodec;
use manic_proto::SymmetricalCodec;
use tokio::net::TcpStream;
use tokio_serde::{Framed, SymmetricallyFramed};
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};

const KEY: &[u8] = &[
    8, 5, 6, 7, 8, 6, 45, 4, 3, 4, 5, 6, 7, 7, 8, 8, 7, 8, 9, 2, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32,
];
#[tokio::main]
pub async fn main() {
    client().await;
}

async fn client() {
    let conn = TcpStream::connect("127.0.0.1:8000").await.unwrap();
    println!("Connected to {:?}", conn.local_addr());

    let len_delim = FramedWrite::new(conn, LengthDelimitedCodec::new());

    let mut ser =
        SymmetricallyFramed::new(len_delim, BincodeCodec::<String, String>::new(KEY.to_vec()));
    println!("Sending");
    println!("Result: {:?}", ser.send("Test".to_string()).await);
    println!("Result2: {:?}", ser.send("Test 2".to_string()).await);
}
