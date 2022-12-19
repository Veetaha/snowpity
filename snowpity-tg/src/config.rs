use crate::{db, derpi, tg};
use serde::de::DeserializeOwned;

pub struct Config {
    pub(crate) tg: tg::Config,
    pub(crate) db: db::Config,
    pub(crate) derpi: derpi::Config,
}

impl Config {
    pub fn load_or_panic() -> Config {
        Self {
            tg: from_env_or_panic("TG_BOT_"),
            db: from_env_or_panic("DATABASE_"),
            derpi: from_env_or_panic("DERPI_"),
        }
    }
}

pub(crate) fn from_env_or_panic<T: DeserializeOwned>(prefix: &str) -> T {
    envy::prefixed(prefix).from_env().unwrap_or_else(|err| {
        panic!(
            "BUG: Couldn't load config from environment for {}: {:#?}",
            std::any::type_name::<T>(),
            err
        );
    })
}
