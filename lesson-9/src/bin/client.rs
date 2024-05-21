use lesson_9::{send_message, Message};
use std::error::Error;
use std::net::TcpStream;
use std::process::exit;
use std::{io, thread};

const ADDRESS: &str = "127.0.0.1";
const PORT: i32 = 11111;

fn main() {
    let mut client =
        TcpStream::connect(format!("{ADDRESS}:{PORT}")).expect("Client failed to connect.");
    client
        .set_nonblocking(true)
        .expect("Failed to make client non-blocking.");

    let _ = thread::spawn(move || loop {
        let input = read_input();
        match input {
            Ok(data) => {
                let message = Message::from_string(data);
                match message {
                    Message::Stop => exit(0),
                    msg => {
                        let _ = send_message(&mut client, &msg);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error sending data to server: {e:?}")
            }
        }
    })
    .join();
}

fn read_input() -> Result<String, Box<dyn Error>> {
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input)?;
    Ok(user_input.trim().to_string())
}
