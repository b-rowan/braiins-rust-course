use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, mpsc, Mutex};
use std::thread;

use lesson_9::{Message, receive_message, send_message};

const ADDRESS: &str = "0.0.0.0";
const PORT: i32 = 11111;

fn main() {
    let server = TcpListener::bind(format!("{ADDRESS}:{PORT}")).expect("Listener failed to bind.");
    server
        .set_nonblocking(true)
        .expect("Failed to make server non-blocking.");

    let (tx_channel, rx_channel) = mpsc::channel::<Message>();

    let clients = Arc::new(Mutex::new(HashMap::<String, TcpStream>::new()));

    let receiver_clients = clients.clone();
    thread::spawn(move || loop {
        let message = rx_channel.recv();
        if let Ok(msg) = message {
            let mut handle = receiver_clients.lock().unwrap();



            for (_, mut client) in handle.iter_mut() {
                send_message(&mut client, &msg);
            }
        }
    });

    loop {
        if let Ok((mut stream, addr)) = server.accept() {
            let tx_local = tx_channel.clone();
            let clients_local = clients.clone();
            clients_local.lock().unwrap().insert(addr.to_string(), stream);
            thread::spawn(move || loop {
                let result = receive_message(&mut clients_local.lock().unwrap().get_mut(&addr.to_string()).unwrap());
                match result {
                    Ok(msg) => {
                        println!("Received message from {addr:?}: {msg:?}");
                        tx_local
                            .send(msg)
                            .expect("Failed to transport message to sender.");
                    }
                    Err(_) => {
                        eprintln!("Closing connection: {addr:?}");
                        // remove stream from clients list
                        clients_local.lock().unwrap().remove(&addr.to_string());
                        break;
                    }
                }
            });
        }
    }
}
