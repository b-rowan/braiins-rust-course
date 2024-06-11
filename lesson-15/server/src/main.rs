use std::{env, io};
use std::fs::File;

use clap::Parser;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::select;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tracing::{event, Level};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{Layer, Registry};
use tracing_subscriber::layer::SubscriberExt;

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

    let (broadcast, _broadcast_recv) = channel::<(String, Message)>(64);
    loop {
        if let Ok((socket, addr)) = server.accept().await {
            println!("Accepted client: {addr}");
            event!(Level::INFO, "Accepted client: {addr}");
            tokio::spawn(handle_client(socket, broadcast.clone()));
        } else {
            println!("Unknown error while accepting new clients.");
            event!(Level::ERROR, "Unknown error while accepting new clients.");
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    broadcast: Sender<(String, Message)>,
) {
    let (stream_recv, stream_write) = stream.into_split();
    let client_send = handle_client_send(stream_write, broadcast.subscribe());
    let client_recv = handle_client_recv(stream_recv, broadcast.clone());

    // if any of these return, they should both be stopped
    let result = select! {
        r = client_send => {
            r
        }
        r = client_recv => {
            r
        }
    };
    if let Err(e) = result {
        if let ServerError::ConnectionClosed(_) = e {
            event!(Level::INFO, "{e}")
        } else {
            event!(Level::ERROR, "{e}")
        }
        println!("{e}");
    }
}

async fn handle_client_send(writer: OwnedWriteHalf, mut broadcast: Receiver<(String, Message)>) -> Result<(), ServerError> {
    let peer_address = writer
        .peer_addr()
        .map_err(|_| ServerError::PeerAddressUnknown)?
        .to_string();
    loop {
        let received = broadcast.recv().await;
        match received {
            Ok(data) => {
                let (address, message) = data;
                if address == peer_address {
                    continue;
                }
                loop {
                    let msg_serialized =
                        serde_cbor::to_vec(&message).map_err(|_| ServerError::MessageSerializeFailed)?;
                    let msg_length = msg_serialized.len() as u32;

                    let write_res = writer.try_write(&msg_length.to_le_bytes());

                    if let Err(e) = write_res {
                        if e.kind() == io::ErrorKind::WouldBlock {
                            continue;
                        }
                        event!(Level::WARN, "Error sending data to {peer_address}: {e}",);
                        break;
                    }
                    writer
                        .try_write(&msg_serialized)
                        .map_err(|_| ServerError::MessageSendFailed(peer_address.clone()))?;
                    break;
                }
            }
            Err(e) => {
                event!(Level::ERROR, "{e}")
            }
        }
    }
}

async fn handle_client_recv(mut reader: OwnedReadHalf, broadcast: Sender<(String, Message)>) -> Result<(), ServerError> {
    let peer_address = reader
        .peer_addr()
        .map_err(|_| ServerError::PeerAddressUnknown)?
        .to_string();
    loop {
        let mut msg_length_raw = [0u8; 4];
        let read_result = reader.read(&mut msg_length_raw).await;
        if let Err(e) = read_result {
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
        reader
            .read_exact(&mut msg_raw)
            .await
            .map_err(|_| ServerError::ReadFailed(peer_address.clone()))?;

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
        };
        broadcast.send((peer_address.clone(), message.clone())).map_err(|_| ServerError::MessageSendFailed(peer_address.clone()))?;
    }
}
