use std::env;
use std::sync::Arc;
use chrono::Utc;
use clap::Parser;
use prometheus::{Gauge, opts, register_gauge};
use rocket::futures::{SinkExt, stream::SplitSink, stream::SplitStream, StreamExt, TryStreamExt};
use rocket_ws::Message as WSMessage;
use rocket_ws::stream::DuplexStream;
use sqlx::SqlitePool;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tracing::{event, Level};

use crate::message::{Message, UserMessage};

use rocket_ws as ws;
use lazy_static::lazy_static;
use crate::Args;

lazy_static! {
    static ref BROADCAST: (Sender<(String, UserMessage)>, Receiver<(String, UserMessage)>) = channel(1024);
    pub static ref MESSAGES_GAUGE: Gauge = register_gauge!(opts!("messages_sent", "The total number of messages sent, including text, photos, and files.")).unwrap();
    pub static ref TEXT_GAUGE: Gauge = register_gauge!(opts!("text_messages_sent", "The total number of text messages sent.")).unwrap();
    pub static ref PHOTOS_GAUGE: Gauge = register_gauge!(opts!("photos_sent", "The total number of photos sent.")).unwrap();
    pub static ref FILES_GAUGE: Gauge = register_gauge!(opts!("files_sent", "The total number of files sent.")).unwrap();
}

async fn ws_recv(key: String, mut recv: SplitStream<DuplexStream>) -> anyhow::Result<()> {
    let broadcast = BROADCAST.0.clone();
    loop {
        if let Ok(data) = recv.try_next().await {
            if data.is_none() {
                return Ok(());
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
async fn ws_send(key: String, mut send: SplitSink<DuplexStream, WSMessage>) -> anyhow::Result<()> {
    let broadcast = BROADCAST.0.clone();
    loop {
        if let Ok(data) = broadcast.subscribe().recv().await {
            if data.0 != key {
                send.send(rocket_ws::Message::Text(
                    serde_json::to_string(&data.1).unwrap(),
                ))
                .await?; }
        }
    }
}


#[get("/ws/chat")]
pub async fn chat_ws(ws: ws::WebSocket) -> ws::Channel<'static> {
    let key = ws.accept_key().to_string();
    ws.channel(move |stream| {
        Box::pin(async move {
            let (send, recv) = stream.split();
            let recv = tokio::spawn(ws_recv(key.clone(), recv));
            let send = tokio::spawn(ws_send(key.clone(), send));

            tokio::join!(recv, send);

            Ok(())
        })
    })
}

async fn handle_msg(message: UserMessage) {
    let local_path = env::current_dir().unwrap();
    let files_path = local_path.join("files");
    let images_path = files_path.join("images");
    let username = message.username.unwrap_or("Anonymous".to_string());
    MESSAGES_GAUGE.inc();
    match message.message {
        Message::File { name, data } => {
            FILES_GAUGE.inc();
            event!(Level::INFO, "Receiving file from \"{username}\": {name}...");
            tokio::fs::write(files_path.clone().join(name), data)
                .await
                .expect("Failed to write received file...");
        }
        Message::Photo { data } => {
            PHOTOS_GAUGE.inc();
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
            TEXT_GAUGE.inc();
            event!(
                Level::INFO,
                "Receiving message from \"{username}\": {message}"
            );
            let args = Args::parse();
            let db = Arc::new(SqlitePool::connect(&args.db_path).await.unwrap());
            sqlx::query("INSERT INTO messages (username, message) VALUES ($1, $2)")
                .bind(&username)
                .bind(message)
                .execute(&mut *db.acquire().await.unwrap())
                .await
                .unwrap();
        }
    }
}
