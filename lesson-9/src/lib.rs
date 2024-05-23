use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;
use image::{DynamicImage, ImageFormat};
use image::io::Reader as ImageReader;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    File { name: String, data: String },
    Photo { data: Vec<u8> },
    Text(String),
    Stop,
}

impl From<String> for Message {
    fn from(value: String) -> Self {
        if value.starts_with(".") {
            // handle command
            let split_data: Vec<_> = value.splitn(2, " ").collect();
            return match split_data[0] {
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
                    Message::Text(value)
                }
                ".image" => {
                    let filename = split_data[1];
                    let file_path = Path::new(filename);

                    let img: DynamicImage = ImageReader::open(file_path).expect("Could not open the specified image.").decode().expect("Could not load the specified image.");
                    let mut buf = Vec::new();
                    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png).unwrap();
                    if file_path.exists() {
                        return Message::Photo {
                            data: buf
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
