use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use parking_lot::FairMutex;

use lesson_9::{receive_message, send_message, Message};

const ADDRESS: &str = "0.0.0.0";
const PORT: i32 = 11111;

fn main() {
    let server = TcpListener::bind(format!("{ADDRESS}:{PORT}")).expect("Listener failed to bind.");
    server
        .set_nonblocking(true)
        .expect("Failed to make server non-blocking.");
    let clients = Arc::new(FairMutex::new(Vec::new()));

    loop {
        if let Ok((stream, addr)) = server.accept() {
            let clients_local = clients.clone();
            thread::spawn(move || handle_client(addr, stream, clients_local));
        }
    }
}

fn handle_client(
    addr: SocketAddr,
    mut stream: TcpStream,
    clients: Arc<FairMutex<Vec<FairMutex<Sender<Message>>>>>,
) {
    stream
        .set_nonblocking(true)
        .expect("Failed to make client stream non-blocking.");
    let (tx, rx) = mpsc::channel::<Message>();
    let idx = {
        let mut handle = clients.lock();
        handle.push(FairMutex::new(tx));
        handle.len() - 1
    };

    let local_stream = Arc::new(FairMutex::new(stream));

    let recv_stream = local_stream.clone();
    thread::spawn(move || loop {
        let result = {
            let mut handle = recv_stream.lock();
            receive_message(&mut handle)
        };

        match result {
            Ok(res) => {
                // handle
                println!("{res:?}");
                let mut handle = clients.lock();
                for c in handle.iter_mut() {
                    c.lock().send(res.clone()).unwrap()
                }
            }
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    eprintln!("Closing connection: {addr:?}");
                    // remove stream from clients list
                    clients.lock().remove(idx);
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(10))
    });

    let send_stream = local_stream.clone();
    thread::spawn(move || loop {
        let new_message = rx.try_recv();

        if new_message.is_ok() {
            let _ = send_message(&mut send_stream.lock(), &new_message.unwrap());
        }
        thread::sleep(Duration::from_millis(15))
    });
}
