#[macro_use]
extern crate rocket;
mod auth;
mod config;
mod pigapi;

use crate::auth::{get_auth_api_routes, OpenIDAuth};
use crate::config::Config;
use crate::pigapi::get_pig_api_routes;
use diesel::{Connection, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use pigweb_common::{AUTH_API_ROOT, PIG_API_ROOT};
use rocket::fairing::AdHoc;
use rocket::fs::FileServer;
use rocket_oauth2::{HyperRustlsAdapter, OAuth2, OAuthConfig};
use std::sync::Mutex;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("data/migrations");

/// /api root path just to verify the backend is online
#[get("/")]
async fn api_root() -> &'static str {
    "             __,---.__
        __,-'         `-.
       /_ /_,'           \\&
       _,''               \\
      (\")            .    |
 api?   ``--|__|--..-'`.__|
\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n
(art by jrei https://ascii.co.uk/art/pig)
"
}

// Start the web sever using the launch macro
#[launch]
async fn rocket() -> _ {
    // Load the config here for the db connection and client path
    let figment = Config::load_figment();
    let config = Config::load_from_figment(&figment);
    let client_path = config.client_path.to_owned();
    let oidc_config = config.oidc.as_ref();

    // Init DB connection
    let connection_str = config.database.to_pg_connection_string();
    let mut db_connection = PgConnection::establish(connection_str.as_str())
        .unwrap_or_else(|e| panic!("Unable to connect to PostgreSQL database {:?}: {:?}", connection_str, e));

    // Run DB migrations, path relative to Cargo.toml
    if db_connection.run_pending_migrations(MIGRATIONS).is_err() {
        panic!("Unable to migrate database to the latest schema.");
    };

    // Init Rocket
    let mut rocket = rocket::custom(figment)
        .manage(Mutex::new(db_connection))
        .attach(AdHoc::config::<Config>())
        .mount("/", FileServer::from(client_path))
        .mount("/api", routes![api_root])
        .mount(AUTH_API_ROOT, get_auth_api_routes())
        .mount(PIG_API_ROOT, get_pig_api_routes());

    // Make sure OAuth2 uses custom config, if defined
    if let Some(oidc_config) = oidc_config {
        rocket =
            rocket.attach(OAuth2::<OpenIDAuth>::custom(HyperRustlsAdapter::default(), oidc_config.to_oauth_config()));
    } else {
        rocket = rocket.attach(OAuth2::<OpenIDAuth>::fairing("generic_oauth2"));
    }

    rocket
}
