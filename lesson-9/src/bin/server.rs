use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, mpsc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use lesson_9::{Message, receive_message, send_message};

const ADDRESS: &str = "0.0.0.0";
const PORT: i32 = 11111;

fn main() {
    let server = TcpListener::bind(format!("{ADDRESS}:{PORT}")).expect("Listener failed to bind.");
    server
        .set_nonblocking(true)
        .expect("Failed to make server non-blocking.");

    let (tx_channel, rx_channel) = mpsc::channel::<Message>();

    let clients = Arc::new(RwLock::new(HashMap::<String, Mutex<TcpStream>>::new()));

    let receiver_clients = clients.clone();
    thread::spawn(move || loop {
        let message = rx_channel.try_recv();
        if let Ok(ref msg) = message {
            let mut handle = receiver_clients.read().unwrap();

            for (_, mut client) in handle.iter() {
                println!("send {msg:?}");
                send_message(client, &msg);
            }
        }
        thread::sleep(Duration::from_millis(1))
    });

    loop {
        if let Ok((stream, addr)) = server.accept() {
            stream.set_nonblocking(true).expect("Failed to make client stream non-blocking.");
            let tx_local = tx_channel.clone();
            let clients_local = clients.clone();
            clients_local.write().unwrap().insert(addr.to_string(), Mutex::new(stream));
            thread::spawn(move || loop {
                let result = {
                    receive_message(&mut clients_local.write().unwrap().get_mut(&addr.to_string()).unwrap())
                };
                match result {
                    Ok(msg) => {
                        println!("Received message from {addr:?}: {msg:?}");
                        tx_local
                            .send(msg)
                            .expect("Failed to transport message to sender.");
                    }
                    Err(e) => {
                        if e.kind() != ErrorKind::WouldBlock {
                            eprintln!("Closing connection: {addr:?}");
                            // remove stream from clients list
                            clients_local.write().unwrap().remove(&addr.to_string());
                            break;
                        }
                        thread::sleep(Duration::from_millis(1))
                    }
                }
            });
        }
    }
}
