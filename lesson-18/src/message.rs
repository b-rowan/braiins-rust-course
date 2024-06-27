use std::{fs, io};
use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, ImageFormat};
use image::io::Reader as ImageReader;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use thiserror::Error;

/// Struct for handling messages from a specific user.
#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct UserMessage {
    pub username: Option<String>,
    pub message: Message,
}

/// Struct for handling different message types.
///
/// # Examples
/// ```
/// use rust_chat::Message;
/// // Should give a Message::Text containing "Hello World"
/// Message::try_from(String::from("Hello World"));
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Message {
    File { name: String, data: Vec<u8> },
    Photo { data: Vec<u8> },
    Text(String),
}

#[derive(Error, Debug)]
pub enum MessageError {
    #[error("File {0} not found.")]
    FileNotFound(String),
    #[error("Failed to read from file.")]
    FileReadFailed(#[from] io::Error),
    #[error("Unsupported image format.")]
    UnsupportedImage(#[from] image::ImageError),
}

impl TryFrom<String> for Message {
    type Error = MessageError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.starts_with(".") {
            // handle command
            let split_data: Vec<_> = value.splitn(2, " ").collect();
            return match split_data[0] {
                ".file" => {
                    let filename = split_data[1];
                    let file_path = Path::new(filename);
                    if !file_path.exists() {
                        return Err(MessageError::FileNotFound(
                            file_path.to_string_lossy().to_string(),
                        ));
                    }
                    return Ok(Message::File {
                        // Unwrap is fine here, we've already checked that it exists
                        name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                        data: fs::read(&filename)?,
                    });
                }
                ".image" => {
                    let filename = split_data[1];
                    let file_path = Path::new(filename);

                    let img: DynamicImage = ImageReader::open(file_path)?.decode()?;
                    let mut buf = Vec::new();
                    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)?;
                    if file_path.exists() {
                        return Ok(Message::Photo { data: buf });
                    };
                    Ok(Message::Text(value))
                }
                _ => Ok(Message::Text(value)),
            };
        }
        Ok(Message::Text(value))
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::Message;

    #[test]
    fn test_text_message() -> Result<(), Box<dyn Error>> {
        let value = String::from("hello");
        let message = Message::try_from(value.clone())?;
        let expected = Message::Text(value.clone());
        assert_eq!(message, expected);
        Ok(())
    }
}
