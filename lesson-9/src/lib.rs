use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    File { name: String, data: String },
    Photo { data: String },
    Text(String),
    Stop,
}

impl Message {
    pub fn from_string(data: String) -> Message {
        if data.starts_with(".") {
            // handle command
            let split_data: Vec<_> = data.splitn(2, " ").collect();
            match split_data[0] {
                ".stop" => Message::Stop,
                ".file" => {
                    let filename = split_data[1];
                    let file_path = Path::new(filename);
                    if file_path.exists() {
                        return Message::File {
                            name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                            data: fs::read_to_string(&filename)
                                .expect("Could not open the specified file."),
                        };
                    }
                    return Message::Text(data);
                }
                ".image" => {
                    let filename = split_data[1];
                    let file_path = Path::new(filename);
                    if file_path.exists() {
                        return Message::Photo {
                            data: fs::read_to_string(&filename)
                                .expect("Could not open the specified file."),
                        };
                    };
                    return Message::Text(data);
                }
                _ => return Message::Text(data),
            };
        }
        Message::Text(data)
    }
}

pub fn send_message(
    stream: &Mutex<TcpStream>,
    message: &Message,
) -> Result<String, Box<dyn Error>> {
    let msg_serialized = serde_cbor::to_vec(message)?;
    let msg_length = msg_serialized.len() as u32;

    let mut s = stream.lock().unwrap();

    // prefix with len
    let _ = &s.write(&msg_length.to_le_bytes());

    // send message
    let _ = &s.write(&msg_serialized)?;

    Ok(String::from("Sent message."))
}

pub fn receive_message(stream: &Mutex<TcpStream>) -> Result<Message, std::io::Error> {
    let mut s = stream.lock().unwrap();

    // get the message length first
    let mut msg_length_raw = [0u8; 4];
    s.read_exact(&mut msg_length_raw)?;

    // read the message based off length
    let msg_length = u32::from_le_bytes(msg_length_raw);
    let mut msg_raw = vec![0u8; usize::try_from(msg_length).unwrap()];
    s.read_exact(&mut msg_raw)?;

    // parse
    let msg: Message = serde_cbor::from_slice(&msg_raw).unwrap();

    Ok(msg)
}
