use std::env;
use std::error::Error;
use std::fs::create_dir_all;
use std::process::exit;
use std::time::Duration;

use async_std::io;
use chrono::Utc;
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use lesson_9::Message;

const ADDRESS: &str = "127.0.0.1";
const PORT: i32 = 11111;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_path = env::current_dir().unwrap();
    let files_path = local_path.join("files");
    let images_path = files_path.join("images");
    create_dir_all(images_path.clone()).unwrap();

    let stream = TcpStream::connect(format!("{ADDRESS}:{PORT}")).await?;
    let (tx, mut rx) = mpsc::channel::<Message>(2048);

    tokio::spawn(async move {
        loop {
            let input = read_input().await.unwrap();
            let message = Message::from(input);

            match message {
                Message::Stop => exit(0),
                m => {
                    tx.send(m).await.unwrap();
                }
            }
        }
    });

    loop {
        let ready = stream
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await?;

        if ready.is_readable() {
            let mut msg_length_raw = [0u8; 4];
            let read_result = stream.try_read(&mut msg_length_raw);
            if read_result.is_err() {
                let result = read_result.err().unwrap();
                if result.kind() == io::ErrorKind::WouldBlock {
                    continue;
                }
                return Err(result.into());
            }

            let msg_length = u32::from_le_bytes(msg_length_raw);
            let mut msg_raw = vec![0u8; usize::try_from(msg_length).unwrap()];
            stream.try_read(&mut msg_raw).unwrap();

            let message: Message = serde_cbor::from_slice(&msg_raw).unwrap();

            match message {
                Message::File { name, data } => {
                    println!("Receiving file: {name}...");
                    tokio::fs::write(files_path.clone().join(name), data)
                        .await
                        .unwrap();
                }
                Message::Photo { data } => {
                    println!("Receiving photo...");
                    let timestamp = Utc::now();
                    tokio::fs::write(images_path.clone().join(format!("{}.png", timestamp.timestamp())), data)
                        .await
                        .unwrap();
                }
                Message::Text(msg) => {
                    println!("{msg}")
                }
                Message::Stop => {}
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
                    return Err(result.into());
                }
                stream.try_write(&msg_serialized).unwrap();
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
