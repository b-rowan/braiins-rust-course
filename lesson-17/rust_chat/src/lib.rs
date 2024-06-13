use std::io::Cursor;
use std::path::Path;
use std::{fs, io};

use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageFormat};
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
    File { name: String, data: String },
    Photo { data: Vec<u8> },
    Text(String),
    SetUser { username: Option<String> },
    Stop,
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
                ".stop" => Ok(Message::Stop),
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
                        data: fs::read_to_string(&filename)?,
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
                ".user" => {
                    return if split_data.len() > 1 {
                        let name = split_data[1];
                        Ok(Message::SetUser {
                            username: Some(name.to_string()),
                        })
                    } else {
                        Ok(Message::SetUser { username: None })
                    }
                }
                _ => Ok(Message::Text(value)),
            };
        }
        Ok(Message::Text(value))
    }
}

#[cfg(test)]
mod tests {
    use crate::Message;
    use std::error::Error;

    #[test]
    fn test_text_message() -> Result<(), Box<dyn Error>> {
        let value = String::from("hello");
        let message = Message::try_from(value.clone())?;
        let expected = Message::Text(value.clone());
        assert_eq!(message, expected);
        Ok(())
    }

    #[test]
    fn test_stop_message() -> Result<(), Box<dyn Error>> {
        let value = String::from(".stop");
        let message = Message::try_from(value.clone())?;
        let expected = Message::Stop;
        assert_eq!(message, expected);
        Ok(())
    }
    #[test]
    fn test_set_user_message() -> Result<(), Box<dyn Error>> {
        let value = String::from(".user Custom");
        let message = Message::try_from(value.clone())?;
        let expected = Message::SetUser {
            username: Some(String::from("Custom")),
        };
        assert_eq!(message, expected);
        Ok(())
    }
    #[test]
    fn test_set_user_empty_message() -> Result<(), Box<dyn Error>> {
        let value = String::from(".user");
        let message = Message::try_from(value.clone())?;
        let expected = Message::SetUser { username: None };
        assert_eq!(message, expected);
        Ok(())
    }
}
