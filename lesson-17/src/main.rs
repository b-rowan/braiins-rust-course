#[macro_use] extern crate rocket;

use std::fs::{create_dir_all, File};
use std::{env, io};
use std::path::Path;
use rocket::fs::{FileServer, NamedFile, relative};
use once_cell::sync::Lazy;
use rocket::futures::{SinkExt, StreamExt, stream::SplitSink, TryStreamExt};
use rocket::futures::stream::SplitStream;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use rocket_ws as ws;
use rocket_ws::stream::DuplexStream;
use rocket_ws::Message as WSMessage;
use anyhow::Result;
use clap::Parser;
use rocket::Config;
use tokio::task::spawn_local;
use tracing::level_filters::LevelFilter;
use tracing::{event, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};
use chrono::Utc;


mod message;
use message::Message;
use crate::message::UserMessage;

/// Struct for parsing args.
#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = String::from("0.0.0.0"))]
    address: String,
    #[arg(short, long, default_value_t = 11111)]
    port: u16,
    #[arg(long, default_value = "info")]
    loglevel: LevelFilter,
    #[arg(long, default_value_t = String::from("server.log"))]
    logfile: String,
    #[arg(long, default_value_t = String::from("sqlite.db"))]
    db_path: String,
}


static BROADCAST: Lazy<(Sender<(String, UserMessage)>, Receiver<(String, UserMessage)>)> = Lazy::new(|| {channel(1024)});

#[get("/")]
async fn index() -> Option<NamedFile> {
    let file_path = Path::new(relative!("pages")).join("index.html");
    NamedFile::open(file_path).await.ok()
}

#[get("/ws/chat")]
async fn chat_ws(ws: ws::WebSocket) -> ws::Channel<'static> {
    let key = ws.accept_key().to_string();
    ws.channel(move | stream| Box::pin(async move {
        let (send, recv) = stream.split();
        let recv = tokio::spawn(ws_recv(key.clone(), recv));
        let send = tokio::spawn(ws_send(key.clone(), send));

        tokio::join!(recv, send);

        Ok(())
    }))
}

async fn ws_recv(key: String, mut recv: SplitStream<DuplexStream>) -> Result<()>{
    let broadcast = BROADCAST.0.clone();
    loop {
        if let Ok(data) = recv.try_next().await {
            if data.is_none() {
                return Ok(())
            }
            let json = serde_json::from_str(data.unwrap().to_text().unwrap());
            if json.is_ok() {
                let message: UserMessage = json.unwrap();
                handle_msg(message.clone()).await;
                broadcast.send((key.clone(), message)).unwrap();
            }
        }
    }
}
async fn ws_send(key: String, mut send: SplitSink<DuplexStream, WSMessage>) -> Result<()> {
    let broadcast = BROADCAST.0.clone();
    loop {
        if let Ok(data) = broadcast.subscribe().recv().await {
            if data.0 != key {
                println!("Sending {:?} to {key}", data.1);
                send.send(rocket_ws::Message::Text(serde_json::to_string(&data.1).unwrap())).await?;
                println!("Sent {:?} to {key}", data.1);
            }
        }
    }
}

async fn handle_msg(message: UserMessage) {
    let local_path = env::current_dir().unwrap();
    let files_path = local_path.join("files");
    let images_path = files_path.join("images");
    let username = message.username.unwrap_or("Anonymous".to_string());
    match message.message {
        Message::File { name, data } => {
            event!(Level::INFO, "Receiving file from \"{username}\": {name}...");
            tokio::fs::write(files_path.clone().join(name), data)
                .await
                .expect("Failed to write received file...");
        }
        Message::Photo { data } => {
            println!("Receiving photo from \"{username}\"...");
            event!(Level::INFO, "Receiving photo from \"{username}\"...");
            let timestamp = Utc::now();
            tokio::fs::write(
                images_path
                    .clone()
                    .join(format!("{}.png", timestamp.timestamp())),
                data,
            )
            .await
            .expect("Failed to write received photo...");
        }
        Message::Text(message) => {
            event!(Level::INFO, "Receiving message from \"{username}\": {message}");
        }
    }
}

#[launch]
fn rocket() -> _ {
    let args = Args::parse();
    let local_path = env::current_dir().unwrap();
    let log_subscriber = Registry::default().with({
        let file =
            File::create(local_path.join(args.logfile)).expect("Failed to create logfile...");
        tracing_subscriber::fmt::layer()
            .with_writer(file)
            .with_writer(io::stdout)
            .with_filter(args.loglevel)
    });
    tracing::subscriber::set_global_default(log_subscriber)
        .expect("Unable to set global subscriber...");
    let files_path = local_path.join("files");
    let images_path = files_path.join("images");

    event!(Level::INFO, "Creating file storage directories...");
    // want to panic here if we can't create the directories, these are required
    create_dir_all(images_path.clone()).expect("Failed to create directories to store files...");
    event!(Level::INFO, "Directories created...");

    let figment = Config::figment().merge(("port", args.port)).merge(("address", args.address));
    rocket::build().configure(figment).mount("/", routes![index, chat_ws]).mount("/files", FileServer::from(files_path))
}
