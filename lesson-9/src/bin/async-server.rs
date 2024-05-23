use std::error::Error;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::FairMutex;
use tokio::io::{AsyncReadExt, Interest};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, UnboundedSender};

use lesson_9::Message;

const ADDRESS: &str = "0.0.0.0";
const PORT: i32 = 11111;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server = TcpListener::bind(format!("{ADDRESS}:{PORT}")).await?;
    let clients = Arc::new(FairMutex::new(Vec::new()));
    loop {
        let (socket, addr) = server.accept().await?;
        println!("Accepted client: {addr}");
        tokio::spawn(handle_client(socket, clients.clone()));
    }
}

async fn handle_client(
    mut stream: TcpStream,
    clients: Arc<FairMutex<Vec<UnboundedSender<Message>>>>,
) {
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    {
        clients.lock().push(tx);
    }
    loop {
        let ready = stream
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await
            .unwrap();

        if ready.is_readable() {
            let mut msg_length_raw = [0u8; 4];
            let read_result = stream.try_read(&mut msg_length_raw);
            if read_result.is_err() {
                let result = read_result.err().unwrap();
                if result.kind() == io::ErrorKind::WouldBlock {
                    continue;
                }
                eprintln!("Error with {}: {result}", stream.peer_addr().unwrap());
                continue;
            }

            let msg_length = u32::from_le_bytes(msg_length_raw);
            let mut msg_raw = vec![0u8; usize::try_from(msg_length).unwrap()];
            stream.read_exact(&mut msg_raw).await.unwrap();

            let message_result = serde_cbor::from_slice(&msg_raw);

            let message: Message = match message_result {
                Ok(msg) => msg,
                Err(_) => {
                    eprintln!(
                        "Client {} closed the connection",
                        stream.peer_addr().unwrap()
                    );
                    return;
                }
            };

            match &message {
                Message::File { name, .. } => {
                    println!(
                        "Receiving file from {}: {name}",
                        stream.peer_addr().unwrap()
                    );
                }
                Message::Photo { .. } => {
                    println!("Receiving photo from {}", stream.peer_addr().unwrap());
                }
                Message::Text(text) => {
                    println!("Got client message: {text}");
                }
                _ => {}
            }

            {
                let client_handle = clients.lock();
                for c in client_handle.iter() {
                    c.send(message.clone()).unwrap();
                }
            }
        }

        let send_data = rx.try_recv();
        if let Ok(data) = send_data {
            loop {
                let msg_serialized = serde_cbor::to_vec(&data).unwrap();
                let msg_length = msg_serialized.len() as u32;

                let write_res = stream.try_write(&msg_length.to_le_bytes());
                if write_res.is_err() {
                    let result = write_res.err().unwrap();
                    if result.kind() == io::ErrorKind::WouldBlock {
                        continue;
                    }
                    eprintln!("Error with {}: {result}", stream.peer_addr().unwrap());
                    break;
                }
                stream.try_write(&msg_serialized).unwrap();
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
