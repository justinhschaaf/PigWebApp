use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use rocket::figment::providers::{Env, Format, Serialized, Toml};
use rocket::figment::Figment;
use rocket_oauth2::{OAuthConfig, StaticProvider};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub client_path: String,
    pub database: DatabaseConfig,
    pub oidc: Option<OpenIDConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config { client_path: "dist".to_owned(), database: Default::default(), oidc: None }
    }
}

impl Config {
    pub fn load() -> Config {
        Self::load_from_figment(&Self::load_figment())
    }

    pub fn load_from_figment(figment: &Figment) -> Config {
        figment.extract().unwrap_or_else(|e| {
            error!("{:?}", e);
            Config::default()
        })
    }

    pub fn load_figment() -> Figment {
        Figment::from(rocket::Config::default())
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file(Env::var_or("PIGWEB_CONFIG", "PigWeb.toml")))
            .merge(Env::prefixed("PIGWEB_").split("__"))
    }
}

// https://www.postgresql.org/docs/9.4/libpq-connect.html#LIBPQ-CONNSTRING
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub uri: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub dbname: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            uri: None,
            host: Some("localhost".to_owned()),
            port: Some(5432),
            dbname: Some("pigweb".to_owned()),
            user: None,
            password: None,
        }
    }
}

impl DatabaseConfig {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenIDConfig {
    pub auth_uri: String,
    pub token_uri: String,
    pub logout_uri: Option<String>,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
}

impl OpenIDConfig {
    pub fn to_oauth_config(&self) -> OAuthConfig {
        OAuthConfig::new(
            StaticProvider {
                auth_uri: Cow::from(self.auth_uri.as_str()),
                token_uri: Cow::from(self.token_uri.as_str()),
            },
            self.client_id.to_owned(),
            self.client_secret.to_owned(),
            None,
        )
    }
}
