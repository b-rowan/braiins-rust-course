use std::env;
use std::error::Error;
use std::fs::{create_dir_all, File};
use std::process::exit;
use std::time::Duration;

use async_std::io;
use chrono::Utc;
use clap::Parser;
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{event, Level};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{prelude::*, Registry};

use rust_chat::Message;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = String::from("127.0.0.1"))]
    address: String,
    #[arg(short, long, default_value_t = 11111)]
    port: u16,
    #[arg(long, default_value = "info")]
    loglevel: LevelFilter,
    #[arg(long, default_value_t = String::from("client.log"))]
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

    let files_path = local_path.join("files");
    let images_path = files_path.join("images");

    event!(Level::INFO, "Creating file storage directories...");
    // want to panic here if we can't create the directories, these are required
    create_dir_all(images_path.clone()).expect("Failed to create directories to store files...");
    event!(Level::INFO, "Directories created...");

    event!(
        Level::INFO,
        "Connecting to server on {}:{}",
        args.address,
        args.port
    );

    // create stream and synchronization channel
    let stream = TcpStream::connect(format!("{}:{}", args.address, args.port)).await?;
    let (tx, mut rx) = mpsc::channel::<Message>(2048);

    // handle user input
    tokio::spawn(async move {
        loop {
            // unrecoverable
            let input = read_input().await.expect("Failed to read input...");
            event!(Level::INFO, "Got input: \"{input}\"");

            let message = Message::try_from(input).unwrap();

            match message {
                Message::Stop => {
                    event!(Level::INFO, "Received stop message, stopping...");
                    exit(0);
                }
                m => {
                    // unrecoverable
                    tx.send(m)
                        .await
                        .expect("Failed to send message to server...");
                }
            }
        }
    });

    // server stream handler, send/receive
    loop {
        // set which types of stream events the client is interested in
        let ready = stream
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await?;

        // check if stream is ready to be read, non-blocking
        if ready.is_readable() {
            let mut msg_length_raw = [0u8; 4];
            let read_result = stream.try_read(&mut msg_length_raw);

            if read_result.is_err() {
                let result = read_result.err().unwrap();
                if result.kind() == io::ErrorKind::WouldBlock {
                    continue;
                }
                event!(Level::ERROR, "Failed to read from server...");
                exit(0);
            }

            let msg_length = u32::from_le_bytes(msg_length_raw);
            let mut msg_raw = vec![
                0u8;
                usize::try_from(msg_length)
                    .expect("Failed to parse message length from server...")
            ];

            stream
                .try_read(&mut msg_raw)
                .expect("Failed to read message from server...");

            let message_result = serde_cbor::from_slice(&msg_raw);

            if message_result.is_err() {
                event!(Level::INFO, "Server disconnected...");
                exit(0);
            }

            let message = message_result.unwrap();

            match message {
                Message::File { name, data } => {
                    println!("Receiving file: {name}...");
                    event!(Level::INFO, "Receiving file: {name}...");
                    tokio::fs::write(files_path.clone().join(name), data)
                        .await
                        .expect("Failed to write received file...");
                }
                Message::Photo { data } => {
                    println!("Receiving photo...");
                    event!(Level::INFO, "Receiving photo...");
                    let timestamp = Utc::now();
                    tokio::fs::write(
                        images_path
                            .clone()
                            .join(format!("{}.png", timestamp.timestamp())),
                        data,
                    )
                    .await
                    .expect("Failed to write received photo...");
                }
                Message::Text(msg) => {
                    println!("{msg}");
                    event!(Level::INFO, "Received message: \"{msg}\"");
                }
                _ => {}
            }
        }

        // check if any data needs to be sent
        let send_data = rx.try_recv();
        if let Ok(data) = send_data {
            loop {
                let msg_serialized =
                    serde_cbor::to_vec(&data).expect("Failed to serialize message for server...");
                let msg_length = msg_serialized.len() as u32;

                event!(Level::INFO, "Sending message to server...");
                let write_res = stream.try_write(&msg_length.to_le_bytes());
                if write_res.is_err() {
                    let result = write_res.err().unwrap();
                    if result.kind() == io::ErrorKind::WouldBlock {
                        continue;
                    }
                    return Err(result.into());
                }
                stream
                    .try_write(&msg_serialized)
                    .expect("Failed to send message to server...");
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn read_input() -> Result<String, Box<dyn Error>> {
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).await?;
    Ok(user_input.trim().to_string())
}
