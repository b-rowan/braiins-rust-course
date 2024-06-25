#[macro_use] extern crate rocket;

use std::path::Path;
use rocket::fs::{NamedFile, relative};


#[get("/")]
async fn index() -> Option<NamedFile> {
    let file_path = Path::new(relative!("pages")).join("index.html");
    println!("{file_path:?}");
    NamedFile::open(file_path).await.ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
