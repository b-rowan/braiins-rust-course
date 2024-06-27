#[macro_use]
extern crate rocket;

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use once_cell::sync::Lazy;
use rocket::fs::{relative, FileServer, NamedFile};
use rocket::futures::stream::SplitStream;
use rocket::futures::{stream::SplitSink, SinkExt, StreamExt, TryStreamExt};
use rocket::http::ext::IntoCollection;
use rocket::response::content;
use rocket::Config;
use rocket_ws as ws;
use rocket_ws::stream::DuplexStream;
use rocket_ws::Message as WSMessage;
use sqlx::sqlite::SqliteRow;
use sqlx::{migrate::MigrateDatabase, Pool, Row, Sqlite, SqlitePool};
use std::fs::{create_dir_all, File};
use std::path::Path;
use std::sync::Arc;
use std::{env, io};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::task::spawn_local;
use tracing::level_filters::LevelFilter;
use tracing::{event, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};

mod message;

use crate::message::UserMessage;
use message::Message;

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

static BROADCAST: Lazy<(
    Sender<(String, UserMessage)>,
    Receiver<(String, UserMessage)>,
)> = Lazy::new(|| channel(1024));

#[get("/")]
async fn index() -> Option<NamedFile> {
    let file_path = Path::new(relative!("pages")).join("index.html");
    NamedFile::open(file_path).await.ok()
}

#[get("/ws/chat")]
async fn chat_ws(ws: ws::WebSocket) -> ws::Channel<'static> {
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

#[get("/api/users")]
async fn api_users() -> content::RawJson<String> {
    let args = Args::parse();
    let db = Arc::new(SqlitePool::connect(&args.db_path).await.unwrap());

    let mut users_data = sqlx::query("SELECT username FROM messages").fetch(&*db);

    let mut users: Vec<String> = Vec::new();
    while let Some(row) = users_data.try_next().await.unwrap() {
        users.push(row.try_get("username").unwrap());
    }

    content::RawJson(serde_json::to_string(&users).unwrap())
}

#[get("/users")]
async fn users_page() -> Option<NamedFile> {
    let file_path = Path::new(relative!("pages")).join("users.html");
    NamedFile::open(file_path).await.ok()
}

#[get("/api/users/delete/<user>")]
async fn delete_user(user: &str) {
    let args = Args::parse();
    let db = Arc::new(SqlitePool::connect(&args.db_path).await.unwrap());

    sqlx::query("DELETE FROM messages WHERE username=$1").bind(user).execute(&*db).await.unwrap();
}

async fn ws_recv(key: String, mut recv: SplitStream<DuplexStream>) -> Result<()> {
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
async fn ws_send(key: String, mut send: SplitSink<DuplexStream, WSMessage>) -> Result<()> {
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

#[launch]
async fn rocket() -> _ {
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

    if !Sqlite::database_exists(&args.db_path)
        .await
        .unwrap_or(false)
    {
        event!(Level::INFO, "Creating message database: {}", &args.db_path);
        Sqlite::create_database(&args.db_path)
            .await
            .expect("Unable to create message database.");
    } else {
        event!(Level::INFO, "Message database exists: {}", &args.db_path);
    }
    let db = Arc::new(SqlitePool::connect(&args.db_path).await.unwrap());
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages \
    (\
        id INTEGER PRIMARY KEY NOT NULL, \
        username VARCHAR(250), \
        message VARCHAR(250) NOT NULL\
    );",
    )
    .execute(&*db)
    .await
    .expect("Failed to set up database.");

    let figment = Config::figment()
        .merge(("port", args.port))
        .merge(("address", args.address));
    rocket::build()
        .configure(figment)
        .mount("/", routes![index, chat_ws, api_users, users_page, delete_user])
        .mount("/files", FileServer::from(files_path))
}
