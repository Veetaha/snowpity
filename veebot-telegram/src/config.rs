use serde::{de::DeserializeOwned, Deserialize};

pub struct Config {
    pub(crate) tg: TgConfig,
    pub(crate) db: DbConfig,
}

#[derive(Deserialize)]
pub(crate) struct TgConfig {
    pub(crate) bot_token: String,
}

#[derive(Deserialize)]
pub(crate) struct DbConfig {
    pub(crate) url: url::Url,

    #[serde(default = "default_database_pool_size")]
    pub(crate) pool_size: u32,
}

fn default_database_pool_size() -> u32 {
    // Free Postgres instances hosted on Heroku have 20 connections limit.
    // However, we also reserve 1 connection for ad-hoc db administration purposes
    // via pg_admin, for example.
    19
}

impl Config {
    pub fn load_or_panic() -> Config {
        Self {
            tg: from_env_or_panic("TELEGRAM_"),
            db: from_env_or_panic("DATABASE_"),
        }
    }
}

fn from_env_or_panic<T: DeserializeOwned>(prefix: &str) -> T {
    envy::prefixed(prefix).from_env().unwrap_or_else(|err| {
        panic!(
            "BUG: Couldn't load config from environment for {}: {:#?}",
            std::any::type_name::<T>(),
            err
        );
    })
}
