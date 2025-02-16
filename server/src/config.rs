use figment::providers::{Env, Format, Toml};
use figment::value::{Dict, Map};
use figment::{Error, Figment, Metadata, Profile, Provider};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub client_path: String,
    pub database: Database,
}

impl Default for Config {
    fn default() -> Self {
        Config { client_path: "dist".to_owned(), database: Default::default() }
    }
}

// Setup Config as a provider for itself, allowing us to easily set defaults
// https://docs.rs/figment/0.10.19/figment/#for-library-authors
impl Provider for Config {
    fn metadata(&self) -> Metadata {
        Metadata::named("Default PigWeb Server config")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        figment::providers::Serialized::defaults(Config::default()).data()
    }

    fn profile(&self) -> Option<Profile> {
        None
    }
}

impl Config {
    pub fn load() -> Config {
        Figment::from(Self::default())
            .merge(Toml::file(Env::var_or("PIGWEB_CONFIG", "PigWeb.toml")))
            .merge(Env::prefixed("PIGWEB_").split("__"))
            .extract()
            .unwrap_or_else(|e| {
                error!("{}", e);
                Config::default()
            })
    }
}

// https://www.postgresql.org/docs/9.4/libpq-connect.html#LIBPQ-CONNSTRING
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub uri: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub dbname: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl Default for Database {
    fn default() -> Self {
        Database {
            uri: None,
            host: Some("localhost".to_owned()),
            port: Some(5432),
            dbname: Some("pigweb".to_owned()),
            user: None,
            password: None,
        }
    }
}

impl Database {
    pub fn to_pg_connection_string(&self) -> String {
        if let Some(uri) = self.uri.to_owned() {
            uri
        } else {
            let mut res = String::new();

            if let Some(host) = self.host.to_owned() {
                res += format!("host='{}' ", host).as_str();
            }

            if let Some(port) = self.port.to_owned() {
                res += format!("port='{:?}' ", port).as_str();
            }

            if let Some(dbname) = self.dbname.to_owned() {
                res += format!("dbname='{}' ", dbname).as_str();
            }

            if let Some(user) = self.user.to_owned() {
                res += format!("user='{}' ", user).as_str();
            }

            if let Some(password) = self.password.to_owned() {
                res += format!("password='{}' ", password).as_str();
            }

            res
        }
    }
}
