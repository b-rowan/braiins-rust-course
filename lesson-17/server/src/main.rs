use std::fs::File;
use std::{env, io};
use std::sync::Arc;

use clap::Parser;
use sqlx::{migrate::MigrateDatabase, Pool, Sqlite, SqlitePool, Row};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tracing::level_filters::LevelFilter;
use tracing::{event, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};

use rust_chat::{Message, UserMessage};

/// Struct for parsing args.
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
    #[arg(long, default_value_t = String::from("sqlite.db"))]
    db_path: String,
}

/// Custom server errors, used internally to communicate error states.
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
    #[error("Failed to write to DB.")]
    DBWriteFailed,
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
            .with_writer(io::stdout)
            .with_filter(args.loglevel)
    });
    tracing::subscriber::set_global_default(log_subscriber)
        .expect("Unable to set global subscriber...");

    let bind_addr = format!("{}:{}", args.address, args.port);

    event!(Level::INFO, "Starting server on {bind_addr}",);
    let server = TcpListener::bind(bind_addr.clone())
        .await
        .expect(&format!("Server failed to bind to {bind_addr}"));
    event!(Level::INFO, "Server serving on {bind_addr}");

    if !Sqlite::database_exists(&args.db_path)
        .await
        .unwrap_or(false)
    {
        event!(Level::INFO, "Creating message database: {}", &args.db_path);
        Sqlite::create_database(&args.db_path)
            .await
            .expect("Unable to create message database.");
    } else {
        event!(Level::INFO, "Message database exists: {}", &args.db_path);
    }
    let db = Arc::new(SqlitePool::connect(&args.db_path).await.unwrap());
    sqlx::query("CREATE TABLE IF NOT EXISTS messages \
    (\
        id INTEGER PRIMARY KEY NOT NULL, \
        username VARCHAR(250), \
        message VARCHAR(250) NOT NULL\
    );").execute(&*db).await.expect("Failed to set up database.");

    // check DB values
    // let result = sqlx::query("SELECT id, username, message FROM messages")
    // .fetch_all(&*db)
    // .await
    // .unwrap();
    // for (idx, row) in result.iter().enumerate() {
    //     println!("[{}]: {}", row.get::<String, &str>("username"), row.get::<String, &str>("message"));
    // }

    let (broadcast, _broadcast_recv) = channel::<(String, UserMessage)>(64);
    loop {
        if let Ok((socket, addr)) = server.accept().await {
            event!(Level::INFO, "Accepted client: {addr}");
            tokio::spawn(handle_client(socket, broadcast.clone(), db.clone()));
        } else {
            event!(Level::ERROR, "Unknown error while accepting new clients.");
        }
    }
}

/// Handle client connections
///
/// This function handles client connections, as well as broadcasting to all the other clients, and writing to the DB.
/// The read anf write loops are handled in subtasks.
async fn handle_client(stream: TcpStream, broadcast: Sender<(String, UserMessage)>, db: Arc<Pool<Sqlite>>) {
    let (stream_recv, stream_write) = stream.into_split();

    let client_send = handle_client_send(stream_write, broadcast.subscribe());
    let client_recv = handle_client_recv(stream_recv, broadcast.clone(), db);

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
    }
}

/// Handles sending data to clients.
async fn handle_client_send(
    writer: OwnedWriteHalf,
    mut broadcast: Receiver<(String, UserMessage)>,
) -> Result<(), ServerError> {
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
                    let msg_serialized = serde_cbor::to_vec(&message)
                        .map_err(|_| ServerError::MessageSerializeFailed)?;
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

/// Handles receiving data from clients.
async fn handle_client_recv(
    mut reader: OwnedReadHalf,
    broadcast: Sender<(String, UserMessage)>,
    db: Arc<Pool<Sqlite>>
) -> Result<(), ServerError> {
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

        let msg = serde_cbor::from_slice::<UserMessage>(&msg_raw)
            .map_err(|_| ServerError::ConnectionClosed(peer_address.clone()))?;

        match &msg.message {
            Message::File { name, .. } => {
                event!(Level::INFO, "Receiving file from {peer_address}: {name}",)
            }
            Message::Photo { .. } => {
                event!(Level::INFO, "Receiving photo from {peer_address}",)
            }
            Message::Text(text) => {
                event!(Level::INFO, "Got message from {peer_address}: {text}");
                sqlx::query("INSERT INTO messages (username, message) VALUES ($1, $2)")
                .bind(&msg.username).bind(text)
                .execute(&mut *db.acquire().await.unwrap())
                .await.map_err(|_| {ServerError::DBWriteFailed})?;
            }
            _ => {}
        };
        broadcast
            .send((peer_address.clone(), msg.clone()))
            .map_err(|_| ServerError::MessageSendFailed(peer_address.clone()))?;
    }
}
