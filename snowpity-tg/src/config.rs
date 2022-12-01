use crate::{db, derpi, tg};
use serde::{de::DeserializeOwned, Deserialize};
use serde_with::serde_as;
use std::collections::HashMap;
use tracing_subscriber::prelude::*;

pub struct Config {
    pub(crate) tg: tg::Config,
    pub(crate) db: db::Config,
    pub(crate) derpi: derpi::Config,
}

impl Config {
    pub fn load_or_panic() -> Config {
        Self {
            tg: from_env_or_panic("TG_"),
            db: from_env_or_panic("DATABASE_"),
            derpi: from_env_or_panic("DERPI_"),
        }
    }
}

#[serde_as]
#[derive(Deserialize)]
pub struct LoggingConfig {
    loki_url: url::Url,
    loki_username: String,
    loki_password: String,
    #[serde_as(as = "serde_with::json::JsonString")]
    tg_bot_log_labels: HashMap<String, String>,
}

impl LoggingConfig {
    pub fn load_or_panic() -> LoggingConfig {
        from_env_or_panic("")
    }

    pub fn init_logging(self) -> tokio::task::JoinHandle<()> {
        let env_filter = tracing_subscriber::EnvFilter::from_env("TG_BOT_LOG");

        let fmt = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(std::env::var("COLORS").as_deref() != Ok("0"))
            .pretty();

        let mut loki_url = self.loki_url.clone();
        loki_url.set_username(&self.loki_username).unwrap();
        loki_url.set_password(Some(&self.loki_password)).unwrap();

        let additional_labels = [
            ("app_version", env!("VERGEN_BUILD_SEMVER")),
            ("app_git_commit", env!("VERGEN_GIT_SHA")),
            ("source", "snowpity-tg"),
        ];

        let mut labels = self.tg_bot_log_labels;
        labels.extend(
            additional_labels
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned())),
        );

        let (loki, task) = tracing_loki::layer(loki_url, labels, HashMap::new()).unwrap();

        let join_handle = tokio::spawn(task);

        tracing_subscriber::registry()
            .with(fmt)
            .with(loki)
            .with(env_filter)
            .init();

        join_handle
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
