use pigweb_common::users::Roles;
use rocket::figment::providers::{Env, Format, Serialized, Toml};
use rocket::figment::Figment;
use rocket_oauth2::{OAuthConfig, StaticProvider};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

/// The config for the PigWeb server.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// The path to the compiled client files
    pub client_path: String,

    /// Config for connecting to the Postgres database
    pub database: DatabaseConfig,

    /// The permission groups the server should recognize.
    ///
    /// The server will read each user's groups when signing in with OIDC and
    /// grant the corresponding roles defined in each group here.
    pub groups: BTreeMap<String, BTreeSet<Roles>>,

    /// Config for the OIDC SSO provider
    pub oidc: Option<OpenIDConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config { client_path: "dist".to_owned(), database: Default::default(), groups: BTreeMap::new(), oidc: None }
    }
}

impl Config {
    /// Loads data from [the default Figment provider](Self::load_figment).
    pub fn load() -> Config {
        Self::load_from_figment(&Self::load_figment())
    }

    /// Loads data from the given Figment provider.
    pub fn load_from_figment(figment: &Figment) -> Config {
        figment.extract().unwrap_or_else(|e| {
            error!("{:?}", e);
            Config::default()
        })
    }

    /// Creates a Figment provider with the default PigWeb and Rocket config
    /// values as a base. Proceeds to load values from the config file (as
    /// defined by the `PIGWEB_CONFIG` env variable, defaults to `PigWeb.toml`)
    /// or env variables prefixed with `PIGWEB_`, nested objects split with `__`
    pub fn load_figment() -> Figment {
        Figment::from(rocket::Config::default())
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file(Env::var_or("PIGWEB_CONFIG", "PigWeb.toml")))
            .merge(Env::prefixed("PIGWEB_").split("__"))
    }
}

/// Params for connecting to the Postgres database
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// The full connection URI to use. If defined, all other options are
    /// ignored and this is used instead. Refer to the
    /// [Postgres docs](https://www.postgresql.org/docs/9.4/libpq-connect.html#LIBPQ-CONNSTRING)
    /// for formatting.
    pub uri: Option<String>,

    /// Name of the host to connect to
    pub host: Option<String>,

    /// Port on the host to connect to
    pub port: Option<u16>,

    /// Name of the database to use
    pub dbname: Option<String>,

    /// The Postgres user to sign in as
    pub user: Option<String>,

    /// The password for the user, if required
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
    /// Generates a Postgres connection string as expected by
    /// [`diesel::PgConnection::establish`], specifically in
    /// [keyword/value format](https://www.postgresql.org/docs/9.4/libpq-connect.html#AEN41757)
    ///
    /// Uses [self.uri] instead if defined.
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

/// Config for the OIDC SSO provider
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenIDConfig {
    /// The endpoint to submit the authorization request to
    pub auth_uri: String,

    /// The token exchange endpoint with the OIDC provider
    pub token_uri: String,

    /// The URI the OIDC provider should send the response to.
    ///
    /// ***Should start with your hostname and end in `/auth/oidc/response`.***
    pub redirect_uri: Option<String>,

    /// When signing out, the user will be redirected here to end the session
    /// with the OIDC provider as well
    pub logout_uri: Option<String>,

    /// The client id assigned by your OIDC provider
    pub client_id: String,

    /// The client secret assigned by your OIDC provider
    pub client_secret: String,

    /// The list of scopes to request from the OIDC provider. Usually just
    /// `openid` and `profile`
    pub scopes: Vec<String>,
}

impl OpenIDConfig {
    /// Converts our OIDC config to an OAuthConfig expected by rocket_oauth2
    pub fn to_oauth_config(&self) -> OAuthConfig {
        OAuthConfig::new(
            StaticProvider {
                auth_uri: Cow::from(self.auth_uri.to_owned()),
                token_uri: Cow::from(self.token_uri.to_owned()),
            },
            self.client_id.to_owned(),
            self.client_secret.to_owned(),
            self.redirect_uri.to_owned(),
        )
    }
}
