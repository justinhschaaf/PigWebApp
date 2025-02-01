#[macro_use]
extern crate rocket;
mod config;
mod pigapi;

use crate::config::Config;
use crate::pigapi::{get_pig_api_routes, TempPigs};
use pigweb_common::PIG_API_ROOT;
use rocket::fs::FileServer;
use std::sync::Mutex;

// Start the web sever using the launch macro
#[launch]
fn rocket() -> _ {
    // Load the config here and convert the client file path to_owned so we can
    // move it to the mutex later.
    let config = Config::load();
    let client_path = config.client_path.to_owned();

    rocket::build()
        .manage(Mutex::new(config))
        .manage(Mutex::new(TempPigs::default()))
        .mount("/", FileServer::from(client_path))
        .mount(PIG_API_ROOT, get_pig_api_routes())
}
