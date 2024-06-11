use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::sync::Arc;
use std::time::Duration;
use std::{env, io};

use clap::Parser;
use parking_lot::FairMutex;
use thiserror::Error;
use tokio::io::{AsyncReadExt, Interest};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, UnboundedSender};
use tracing::level_filters::LevelFilter;
use tracing::{event, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};

use rust_chat::Message;

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

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Could not get peer address.")]
    PeerAddressUnknown,
    #[error("Failed to initialize read/write checks for client {0}.")]
    ReadWriteInitFailed(String),
    #[error("Failed to read valid data from client {0}.")]
    ReadFailed(String),
    #[error("Client {0} closed the connection.")]
    ConnectionClosed(String),
    #[error("Failed to send message to client {0}.")]
    MessageSendFailed(String),
    #[error("Failed to serialize message.")]
    MessageSerializeFailed,
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
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

    let bind_addr = format!("{}:{}", args.address, args.port);

    event!(Level::INFO, "Starting server on {bind_addr}",);
    let server = TcpListener::bind(bind_addr.clone())
        .await
        .expect(&format!("Server failed to bind to {bind_addr}"));
    println!("Server serving on {bind_addr}");
    event!(Level::INFO, "Server serving on {bind_addr}");

    let clients = Arc::new(FairMutex::new(HashMap::new()));
    loop {
        if let Ok((socket, addr)) = server.accept().await {
            println!("Accepted client: {addr}");
            event!(Level::INFO, "Accepted client: {addr}");
            tokio::spawn(handle_client(socket, clients.clone()));
        } else {
            println!("Unknown error while accepting new clients.");
            event!(Level::ERROR, "Unknown error while accepting new clients.");
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    clients: Arc<FairMutex<HashMap<String, UnboundedSender<Message>>>>,
) {
    let peer_addr = if let Ok(addr) = stream.peer_addr() {
        addr.to_string()
    } else {
        event!(
            Level::ERROR,
            "Could not get peer address when setting up client handler."
        );
        return;
    };

    if let Err(e) = _handle_client(stream, &clients).await {
        match e {
            ServerError::ConnectionClosed(_) => event!(Level::INFO, "{e}"),
            _ => event!(Level::INFO, "{e}"),
        };
        println!("{e}");
    }
    clients.lock().remove(&peer_addr);
}

async fn _handle_client(
    mut stream: TcpStream,
    clients: &Arc<FairMutex<HashMap<String, UnboundedSender<Message>>>>,
) -> Result<(), ServerError> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let peer_address = stream
        .peer_addr()
        .map_err(|_| ServerError::PeerAddressUnknown)?
        .to_string();
    clients.lock().insert(peer_address.clone(), tx);

    loop {
        let ready = stream
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await
            .map_err(|_| ServerError::ReadWriteInitFailed(peer_address.clone()))?;

        // check if stream is ready to be read from, non-blocking
        if ready.is_readable() {
            let mut msg_length_raw = [0u8; 4];
            let read_result = stream.try_read(&mut msg_length_raw);

            if let Err(e) = read_result {
                if e.kind() == io::ErrorKind::WouldBlock {
                    continue;
                }
                event!(Level::WARN, "Error with {peer_address}: {e}",);
                continue;
            }

            let msg_length = u32::from_le_bytes(msg_length_raw);
            let msg_len = if let Ok(len) = usize::try_from(msg_length) {
                len
            } else {
                event!(
                    Level::WARN,
                    "Failed to convert message length for {peer_address}."
                );
                continue;
            };
            let mut msg_raw = vec![0u8; msg_len];
            stream
                .read_exact(&mut msg_raw)
                .await
                .map_err(|e| ServerError::ReadFailed(peer_address.clone()))?;

            let message = serde_cbor::from_slice(&msg_raw)
                .map_err(|_| ServerError::ConnectionClosed(peer_address.clone()))?;

            match &message {
                Message::File { name, .. } => {
                    event!(Level::INFO, "Receiving file from {peer_address}: {name}",)
                }
                Message::Photo { .. } => {
                    event!(Level::INFO, "Receiving photo from {peer_address}",)
                }
                Message::Text(text) => {
                    event!(Level::INFO, "Got message from {peer_address}: {text}")
                }
                _ => {}
            }

            {
                let client_handle = clients.lock();
                for c in client_handle.values() {
                    c.send(message.clone())
                        .map_err(|_| ServerError::MessageSendFailed(peer_address.clone()))?;
                }
            }
        }

        let send_data = rx.try_recv();
        if let Ok(data) = send_data {
            loop {
                let msg_serialized =
                    serde_cbor::to_vec(&data).map_err(|_| ServerError::MessageSerializeFailed)?;
                let msg_length = msg_serialized.len() as u32;

                let write_res = stream.try_write(&msg_length.to_le_bytes());

                if let Err(e) = write_res {
                    if e.kind() == io::ErrorKind::WouldBlock {
                        continue;
                    }
                    event!(Level::WARN, "Error sending data to {peer_address}: {e}",);
                    break;
                }
                stream
                    .try_write(&msg_serialized)
                    .map_err(|_| ServerError::MessageSendFailed(peer_address.clone()))?;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
