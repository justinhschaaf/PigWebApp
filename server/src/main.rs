#[macro_use]
extern crate rocket;
mod pigapi;

use crate::pigapi::{get_pig_api_routes, TempPigs};
use rocket::fs::NamedFile;
use rocket::response::status::NotFound;
use std::path::PathBuf;
use std::sync::Mutex;

// Create a route for any url relative to /
// https://theadventuresofaliceandbob.com/posts/rust_rocket_yew_part1.md
#[get("/<path..>")]
async fn static_files(path: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = PathBuf::from("dist").join(path);
    match NamedFile::open(path).await {
        Ok(f) => Ok(f),
        Err(_) => index().await, // If no file is found, route to index
    }
}

// Set the index route
// https://theadventuresofaliceandbob.com/posts/rust_rocket_yew_part1.md
#[get("/")]
async fn index() -> Result<NamedFile, NotFound<String>> {
    NamedFile::open("dist/index.html").await.map_err(|e| NotFound(e.to_string()))
}

#[get("/api")]
async fn api() -> &'static str {
    "Hello World"
}

// Start the web sever using the launch macro
#[launch]
fn rocket() -> _ {
    // TODO implement better logging with log
    rocket::build()
        .manage(Mutex::new(TempPigs::default()))
        .mount("/", routes![index, static_files, api])
        .mount("/api/pigs/", get_pig_api_routes())
}
