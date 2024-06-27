#[macro_use]
extern crate rocket;

use std::{env, io};
use std::fs::{create_dir_all, File};
use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use rocket::Config;
use rocket::fs::{FileServer, NamedFile, relative};
use rocket::futures::TryStreamExt;
use rocket::response::content;
use rocket_prometheus::PrometheusMetrics;
use sqlx::{migrate::MigrateDatabase, Row, Sqlite, SqlitePool};
use tracing::{event, Level};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{Layer, Registry};
use tracing_subscriber::layer::SubscriberExt;

mod message;
mod ws;

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


#[get("/")]
async fn index() -> Option<NamedFile> {
    let file_path = Path::new(relative!("pages")).join("index.html");
    NamedFile::open(file_path).await.ok()
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

    let prometheus = PrometheusMetrics::new();
    prometheus.registry().register(Box::new(ws::MESSAGES_GAUGE.clone())).unwrap();
    prometheus.registry().register(Box::new(ws::TEXT_GAUGE.clone())).unwrap();
    prometheus.registry().register(Box::new(ws::PHOTOS_GAUGE.clone())).unwrap();
    prometheus.registry().register(Box::new(ws::FILES_GAUGE.clone())).unwrap();
    let chat_figment = Config::figment()
        .merge(("port", args.port))
        .merge(("address", args.address.clone()));

    rocket::build()
        .configure(chat_figment)
        .attach(prometheus.clone())
        .mount("/", routes![index, ws::chat_ws, api_users, users_page, delete_user])
        .mount("/files", FileServer::from(files_path))
        .mount("/metrics", prometheus)
}
