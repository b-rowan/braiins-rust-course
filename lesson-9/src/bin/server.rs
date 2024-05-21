use std::net::TcpListener;
use std::thread;

use lesson_9::receive_message;

const ADDRESS: &str = "0.0.0.0";
const PORT: i32 = 11111;

fn main() {
    let server = TcpListener::bind(format!("{ADDRESS}:{PORT}")).expect("Listener failed to bind.");
    server
        .set_nonblocking(true)
        .expect("Failed to make server non-blocking.");

    loop {
        if let Ok((mut stream, addr)) = server.accept() {
            thread::spawn(move || loop {
                let result = receive_message(&mut stream);
                match result {
                    Ok(msg) => {
                        println!("{msg:?}")
                    }
                    Err(_) => {
                        eprintln!("Closing connection: {addr:?}");
                        break;
                    }
                }
            });
        }
    }
}
