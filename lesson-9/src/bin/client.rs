use lesson_9::{receive_message, send_message, Message};
use std::error::Error;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::process::exit;
use std::sync::mpsc;
use std::time::Duration;
use std::{io, thread};

const ADDRESS: &str = "127.0.0.1";
const PORT: i32 = 11111;

fn main() {
    let mut client =
        TcpStream::connect(format!("{ADDRESS}:{PORT}")).expect("Client failed to connect.");
    client
        .set_nonblocking(true)
        .expect("Failed to make client non-blocking.");

    let (tx, rx) = mpsc::channel::<Message>();

    thread::spawn(move || loop {
        let input = read_input();
        match input {
            Ok(data) => {
                let message = Message::from_string(data);
                match message {
                    Message::Stop => exit(0),
                    msg => {
                        tx.send(msg);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error handling message: {e:?}")
            }
        }
        thread::sleep(Duration::from_millis(10))
    });

    loop {
        let result = receive_message(&mut client);

        match result {
            Ok(res) => {
                println!("{res:?}")
            }
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    eprintln!("Client error: {e:?}");
                    break;
                }
            }
        }

        let msg = rx.try_recv();

        if msg.is_ok() {
            send_message(&mut client, &msg.unwrap()).unwrap();
        }
        thread::sleep(Duration::from_millis(15));
    }
}

fn read_input() -> Result<String, Box<dyn Error>> {
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input)?;
    Ok(user_input.trim().to_string())
}
