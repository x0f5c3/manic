use futures::Stream;
use futures::StreamExt;
use manic_proto::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinError;
use tokio_serde::{Framed, SymmetricallyFramed};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};

const KEY: &[u8] = &[
    8, 5, 6, 7, 8, 6, 45, 4, 3, 4, 5, 6, 7, 7, 8, 8, 7, 8, 9, 2, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32,
];

#[tokio::main]
pub async fn main() {
    server().await;
}
async fn server() {
    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    println!("listening on {:?}", listener.local_addr());

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        println!("Client connected: {:?}", socket.local_addr());

        // Delimit frames using a length header
        let length_delimited = FramedRead::new(socket, LengthDelimitedCodec::new());

        // Deserialize frames
        let mut deserialized: Framed<
            FramedRead<tokio::net::TcpStream, LengthDelimitedCodec>,
            String,
            String,
            EncryptedBincode<String, String>,
        > = tokio_serde::SymmetricallyFramed::new(
            length_delimited,
            SymmetricalEncryptedBincode::<String>::new(KEY.to_vec()),
        );

        // Spawn a task that prints all received messages to STDOUT
        tokio::spawn(async move {
            while let Some(msg) = deserialized.next().await {
                println!("GOT: {:?}", msg);
            }
        });
    }
}
