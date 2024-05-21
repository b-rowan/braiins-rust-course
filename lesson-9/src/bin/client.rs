use lesson_9::{receive_message, send_message, Message};
use std::error::Error;
use std::net::TcpStream;
use std::process::exit;
use std::{io, thread};
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};

const ADDRESS: &str = "127.0.0.1";
const PORT: i32 = 11111;

fn main() {
    let mut client =
        Arc::new(Mutex::new(TcpStream::connect(format!("{ADDRESS}:{PORT}")).expect("Client failed to connect.")));
    client
        .lock().unwrap().set_nonblocking(true)
        .expect("Failed to make client non-blocking.");

    let receiver_client = client.clone();
    thread::spawn(move || loop {
        let input = read_input();
        match input {
            Ok(data) => {
                let message = Message::from_string(data);
                match message {
                    Message::Stop => exit(0),
                    msg => {
                        let _ = send_message(&receiver_client, &msg);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error sending data to server: {e:?}")
            }
        }
    });
    loop {
        let result = receive_message(&client);

        match result {
            Ok(res) => {println!("{res:?}")}
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    eprintln!("Client error: {e:?}");
                    break
                }
            }
        }
    }
}

fn read_input() -> Result<String, Box<dyn Error>> {
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input)?;
    Ok(user_input.trim().to_string())
}
