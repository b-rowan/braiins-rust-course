use lesson_9::Message;
use std::error::Error;
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use tokio::io::{AsyncReadExt, Interest};
use tokio::net::TcpStream;

const ADDRESS: &str = "0.0.0.0";
const PORT: i32 = 11111;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let stream = TcpStream::connect(format!("{ADDRESS}:{PORT}")).await?;
    let (tx, rx) = mpsc::channel::<Message>();

    tokio::spawn(async move {
        loop {
            let input = read_input();

            tx.send(Message::from(input.unwrap())).unwrap();
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
            println!("{message:?}")
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
    }
}

fn read_input() -> Result<String, Box<dyn Error>> {
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input)?;
    Ok(user_input.trim().to_string())
}
