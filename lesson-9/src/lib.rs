use std::fs;
use std::io::Read;
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
            return match split_data[0] {
                ".stop" => { Message::Stop },
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
                    Message::Text(value)
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
                    Message::Text(value)
                }
                _ => Message::Text(value),
            };
        }
        Message::Text(value)
    }
}
