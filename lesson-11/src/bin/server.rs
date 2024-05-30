use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::sync::Arc;
use std::time::Duration;
use std::{env, io};

use clap::Parser;
use parking_lot::FairMutex;
use tokio::io::{AsyncReadExt, Interest};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, UnboundedSender};
use tracing::level_filters::LevelFilter;
use tracing::{event, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};

use lesson_11::Message;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = String::from("0.0.0.0"))]
    address: String,
    #[arg(short, long, default_value_t = 11111)]
    port: u16,
    #[arg(long, default_value = "info")]
    loglevel: LevelFilter,
    #[arg(long, default_value_t = String::from("server.log"))]
    logfile: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let local_path = env::current_dir().unwrap();

    // initialize logging
    let log_subscriber = Registry::default().with({
        let file =
            File::create(local_path.join(args.logfile)).expect("Failed to create logfile...");
        tracing_subscriber::fmt::layer()
            .with_writer(file)
            .with_filter(args.loglevel)
    });
    tracing::subscriber::set_global_default(log_subscriber)
        .expect("Unable to set global subscriber...");

    event!(
        Level::INFO,
        "Starting server on {}:{}",
        args.address,
        args.port
    );
    let server = TcpListener::bind(format!("{}:{}", args.address, args.port)).await?;
    println!("Server serving on {}:{}", args.address, args.port);
    event!(
        Level::INFO,
        "Server serving on {}:{}",
        args.address,
        args.port
    );

    let clients = Arc::new(FairMutex::new(HashMap::new()));
    loop {
        let (socket, addr) = server.accept().await?;
        println!("Accepted client: {addr}");
        event!(Level::INFO, "Accepted client: {addr}");
        tokio::spawn(handle_client(socket, clients.clone()));
    }
}

async fn handle_client(
    mut stream: TcpStream,
    clients: Arc<FairMutex<HashMap<String, UnboundedSender<Message>>>>,
) {
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    {
        clients
            .lock()
            .insert(stream.peer_addr().unwrap().to_string(), tx);
    }
    loop {
        let ready = stream
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await
            .expect("Failed to initialized read/write checks...");


        // check if stream is ready to be read from, non-blocking
        if ready.is_readable() {
            let mut msg_length_raw = [0u8; 4];
            let read_result = stream.try_read(&mut msg_length_raw);
            if read_result.is_err() {
                let result = read_result.err().unwrap();
                if result.kind() == io::ErrorKind::WouldBlock {
                    continue;
                }
                event!(Level::WARN, "Error with {}: {result}", stream.peer_addr().unwrap());
                continue;
            }

            let msg_length = u32::from_le_bytes(msg_length_raw);
            let mut msg_raw = vec![
                0u8;
                usize::try_from(msg_length)
                    .expect("Failed to parse message length from client...")
            ];
            stream
                .read_exact(&mut msg_raw)
                .await
                .expect("Failed to read message from client...");

            let message_result = serde_cbor::from_slice(&msg_raw);

            let message: Message = match message_result {
                Ok(msg) => msg,
                Err(_) => {
                    event!(
                        Level::INFO,
                        "Client {} closed the connection",
                        stream.peer_addr().unwrap()
                    );
                    clients
                        .lock()
                        .remove(&stream.peer_addr().unwrap().to_string());
                    return;
                }
            };

            match &message {
                Message::File { name, .. } => {
                    event!(Level::INFO,
                        "Receiving file from {}: {name}",
                        stream.peer_addr().unwrap()
                    );
                }
                Message::Photo { .. } => {
                    event!(Level::INFO, "Receiving photo from {}", stream.peer_addr().unwrap());
                }
                Message::Text(text) => {
                    event!(Level::INFO, "Got client message: {text}");
                }
                _ => {}
            }

            {
                let client_handle = clients.lock();
                for c in client_handle.values() {
                    c.send(message.clone())
                        .expect("Failed to broadcast message from client...");
                }
            }
        }

        let send_data = rx.try_recv();
        if let Ok(data) = send_data {
            loop {
                let msg_serialized =
                    serde_cbor::to_vec(&data).expect("Failed to serialize message for client...");
                let msg_length = msg_serialized.len() as u32;

                let write_res = stream.try_write(&msg_length.to_le_bytes());
                if write_res.is_err() {
                    let result = write_res.err().unwrap();
                    if result.kind() == io::ErrorKind::WouldBlock {
                        continue;
                    }
                    event!(Level::WARN, "Error with {}: {result}", stream.peer_addr().unwrap());
                    break;
                }
                stream
                    .try_write(&msg_serialized)
                    .expect("Failed to send message to client...");
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
