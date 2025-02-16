#[macro_use]
extern crate rocket;
mod config;
mod pigapi;

use crate::config::Config;
use crate::pigapi::get_pig_api_routes;
use diesel::{Connection, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use pigweb_common::PIG_API_ROOT;
use rocket::fs::FileServer;
use std::sync::Mutex;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("data/migrations");

// Start the web sever using the launch macro
#[launch]
async fn rocket() -> _ {
    // Load the config here and convert the client file path to_owned so we can
    // move it to the mutex later.
    let config = Config::load();
    let client_path = config.client_path.to_owned();

    // Init DB connection
    let connection_str = config.database.to_pg_connection_string();
    let mut db_connection = PgConnection::establish(connection_str.as_str())
        .unwrap_or_else(|e| panic!("Unable to connect to PostgreSQL database {:?}: {:?}", connection_str, e));

    // Run DB migrations, path relative to Cargo.toml
    if db_connection.run_pending_migrations(MIGRATIONS).is_err() {
        panic!("Unable to migrate database to the latest schema.");
    };

    // Init Rocket
    rocket::build()
        .manage(Mutex::new(config))
        .manage(Mutex::new(db_connection))
        .mount("/", FileServer::from(client_path))
        .mount(PIG_API_ROOT, get_pig_api_routes())
}
