use figment::providers::{Env, Format, Toml};
use figment::value::{Dict, Map};
use figment::{Error, Figment, Metadata, Profile, Provider};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Config {}

impl Default for Config {
    fn default() -> Self {
        Config {}
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
            .merge(Env::prefixed("PIGWEB_"))
            .extract()
            .unwrap_or_else(|e| {
                error!("{}", e);
                Config::default()
            })
    }
}
