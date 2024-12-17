#[macro_use]
extern crate rocket;

use pigweb_common::Pig;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::response::status::{Created, NotFound};
use rocket::serde::json::Json;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, PartialEq, FromForm)]
struct PigQuery<'r> {
    // Option is necessary to make it so both args aren't absolutely required
    id: Option<Vec<&'r str>>,
    name: Option<&'r str>,
}

// Create a route for any url relative to /
// https://theadventuresofaliceandbob.com/posts/rust_rocket_yew_part1.md
#[get("/<path..>")]
async fn static_files(path: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = PathBuf::from("../client/dist").join(path);
    match NamedFile::open(path).await {
        Ok(f) => Ok(f),
        Err(_) => index().await, // If no file is found, route to index
    }
}

// Set the index route
// https://theadventuresofaliceandbob.com/posts/rust_rocket_yew_part1.md
#[get("/")]
async fn index() -> Result<NamedFile, NotFound<String>> {
    NamedFile::open("../client/dist/index.html").await.map_err(|e| NotFound(e.to_string()))
}

#[get("/api")]
async fn api() -> &'static str {
    "Hello World"
}

#[post("/api/pigs/create?<name>")]
async fn api_pig_create(name: &str) -> Result<Created<Json<Pig<'_>>>, (Status, &'static str)> {
    // Server should generate a UUID, determine the creating user and timestamp, save it to the DB, and return the final object
    Err((Status::NoContent, "Not yet implemented!"))
}

#[put("/api/pigs/update", data = "<pig>")]
async fn api_pig_update(pig: Json<Pig<'_>>) -> (Status, &'static str) {
    (Status::NoContent, "Not yet implemented!")
}

#[delete("/api/pigs/delete?<id>")]
async fn api_pig_delete(id: &str) -> (Status, &'static str) {
    (Status::NoContent, "Not yet implemented!")
}

#[get("/api/pigs/fetch?<query..>")]
async fn api_pig_fetch(query: PigQuery<'_>) -> Result<Json<Vec<Pig<'_>>>, (Status, &'static str)> {
    // Name should be a search with Tantivy, ID should fetch pigs by their ID
    // Add a limit to the number of results? we don't wanna return the whole fucking database
    Err((Status::NoContent, "Not yet implemented!"))
}

// Start the web sever using the launch macro
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, static_files, api, api_pig_create, api_pig_update, api_pig_delete, api_pig_fetch])
}
