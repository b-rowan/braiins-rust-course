use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    File { name: String, data: String },
    Photo { data: String },
    Text(String),
    Stop,
}

impl From<String> for Message {
    fn from(value: String) -> Self {
        if value.starts_with(".") {
            // handle command
            let split_data: Vec<_> = value.splitn(2, " ").collect();
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
                    return Message::Text(value);
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
                    return Message::Text(value);
                }
                _ => return Message::Text(value),
            };
        }
        Message::Text(value)
    }
}

pub fn send_message(stream: &mut TcpStream, message: &Message) -> Result<String, Box<dyn Error>> {
    let msg_serialized = serde_cbor::to_vec(message)?;
    let msg_length = msg_serialized.len() as u32;

    // prefix with len
    let _ = stream.write(&msg_length.to_le_bytes());

    // send message
    let _ = stream.write(&msg_serialized)?;

    Ok(String::from("Sent message."))
}

pub fn receive_message(stream: &mut TcpStream) -> Result<Message, std::io::Error> {
    // get the message length first
    let mut msg_length_raw = [0u8; 4];
    stream.read_exact(&mut msg_length_raw)?;

    // read the message based off length
    let msg_length = u32::from_le_bytes(msg_length_raw);
    let mut msg_raw = vec![0u8; usize::try_from(msg_length).unwrap()];
    stream.read_exact(&mut msg_raw)?;

    // parse
    let msg: Message = serde_cbor::from_slice(&msg_raw).unwrap();

    Ok(msg)
}
