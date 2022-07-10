use serde::{de::DeserializeOwned, Deserialize};
use std::collections::HashMap;
use teloxide::types::UserId;
use tracing_subscriber::prelude::*;

pub struct Config {
    pub(crate) tg: TgConfig,
    pub(crate) db: DbConfig,
}

#[derive(Deserialize, Clone)]
pub(crate) struct TgConfig {
    pub(crate) bot_token: String,

    /// ID of the user, who owns the bot, and thus has full access to it
    pub(crate) bot_maintainer: UserId,
}

#[derive(Deserialize)]
pub(crate) struct DbConfig {
    pub(crate) url: url::Url,

    #[serde(default = "default_database_pool_size")]
    pub(crate) pool_size: u32,
}

fn default_database_pool_size() -> u32 {
    // Postgres instance has 100 connections limit.
    // However, we also reserve 2 connections for ad-hoc db administration purposes
    // via pg_admin, for example.
    98
}

impl Config {
    pub fn load_or_panic() -> Config {
        Self {
            tg: from_env_or_panic("TG_"),
            db: from_env_or_panic("DATABASE_"),
        }
    }
}

#[derive(Deserialize)]
pub struct LoggingConfig {
    loki_url: url::Url,
    loki_username: String,
    loki_password: String,
    #[serde(with = "serde_with::json::nested")]
    veebot_log_labels: HashMap<String, String>,
}

impl LoggingConfig {
    pub fn load_or_panic() -> LoggingConfig {
        from_env_or_panic("")
    }

    pub fn init_logging(self) {
        let env_filter = tracing_subscriber::EnvFilter::from_env("VEEBOT_LOG");

        let fmt = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(std::env::var("COLORS").as_deref() != Ok("0"))
            .pretty();

        let mut loki_url = self.loki_url.clone();
        loki_url.set_username(&self.loki_username).unwrap();
        loki_url.set_password(Some(&self.loki_password)).unwrap();

        let mut labels = self.veebot_log_labels;

        labels.insert("source".to_owned(), "veebot".to_owned());

        let (loki, task) = tracing_loki::layer(loki_url, labels, HashMap::new()).unwrap();

        tokio::spawn(task);

        tracing_subscriber::registry()
            .with(fmt)
            .with(loki)
            .with(env_filter)
            .init();
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
