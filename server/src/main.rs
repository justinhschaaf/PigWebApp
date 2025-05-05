#[macro_use]
extern crate rocket;

mod auth;
mod bulkapi;
mod config;
mod pigapi;
mod userapi;

use crate::auth::get_auth_api_routes;
use crate::bulkapi::get_bulk_api_routes;
use crate::config::Config;
use crate::pigapi::get_pig_api_routes;
use crate::userapi::get_user_api_routes;
use diesel::{Connection, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use pigweb_common::{OpenIDAuth, AUTH_API_ROOT, BULK_API_ROOT, PIG_API_ROOT, USER_API_ROOT};
use rocket::fairing::AdHoc;
use rocket::fs::NamedFile;
use rocket::response::status::NotFound;
use rocket::State;
use rocket_oauth2::{HyperRustlsAdapter, OAuth2};
use std::path::PathBuf;
use std::sync::Mutex;

/// Embeds all migrations to set up the Postgres database in the app binary
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("data/migrations");

/// Create a route for any url relative to the website root. If not found,
/// redirect to index. Rank must be higher than the index route.
/// from https://theadventuresofaliceandbob.com/posts/rust_rocket_yew_part1.md
#[get("/<path..>", rank = 1001)]
async fn files(config: &State<Config>, path: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = PathBuf::from(&config.client_path).join(path);
    match NamedFile::open(path).await {
        Ok(f) => Ok(f),
        Err(_) => index(config).await, // If no file is found, route to index
    }
}

/// Serve the index file
/// from https://theadventuresofaliceandbob.com/posts/rust_rocket_yew_part1.md
#[get("/", rank = 1000)]
async fn index(config: &State<Config>) -> Result<NamedFile, NotFound<String>> {
    NamedFile::open(PathBuf::from(&config.client_path).join("index.html")).await.map_err(|e| NotFound(e.to_string()))
}

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

/// Starts the web sever
#[launch]
async fn rocket() -> _ {
    // Load the config here for the db connection and client path
    let figment = Config::load_figment();
    let config = Config::load_from_figment(&figment);
    let oidc_config = config.oidc.as_ref();

    // Init DB connection
    let connection_str = config.database.to_pg_connection_string();
    let mut db_connection = PgConnection::establish(connection_str.as_str())
        .unwrap_or_else(|e| panic!("Unable to connect to PostgreSQL database {:?}: {:?}", connection_str, e));

    // Run DB migrations, path relative to Cargo.toml
    if db_connection.run_pending_migrations(MIGRATIONS).is_err() {
        panic!("Unable to migrate database to the latest schema.");
    };

    // warn if groups are not configured
    if config.groups.is_empty() {
        warn!("No permission groups have been configured. All users will have all permissions, I hope you know what you're doing!!!")
    }

    // Init Rocket
    let mut rocket = rocket::custom(figment)
        .manage(Mutex::new(db_connection))
        .attach(AdHoc::config::<Config>())
        .mount("/", routes![index, files])
        .mount("/api", routes![api_root])
        .mount(AUTH_API_ROOT, get_auth_api_routes())
        .mount(BULK_API_ROOT, get_bulk_api_routes())
        .mount(PIG_API_ROOT, get_pig_api_routes())
        .mount(USER_API_ROOT, get_user_api_routes());

    // Make sure OAuth2 uses custom config, if defined
    if let Some(oidc_config) = oidc_config {
        rocket =
            rocket.attach(OAuth2::<OpenIDAuth>::custom(HyperRustlsAdapter::default(), oidc_config.to_oauth_config()));
    } else {
        warn!("Unable to find OIDC configuration. This is not supported, use at your own risk!!!");
        rocket = rocket.attach(OAuth2::<OpenIDAuth>::fairing("generic_oauth2"));
    }

    rocket
}
