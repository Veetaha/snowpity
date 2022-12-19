use super::GLOBAL_LABELS;
use crate::config::from_env_or_panic;
use serde::Deserialize;
use serde_with::serde_as;
use std::collections::HashMap;
use tracing_subscriber::prelude::*;

pub fn init_logging() -> tokio::task::JoinHandle<()> {
    LoggingConfig::load_or_panic().init_logging()
}

#[serde_as]
#[derive(Deserialize)]
struct LoggingConfig {
    loki_url: url::Url,
    #[serde_as(as = "serde_with::json::JsonString")]
    tg_bot_log_labels: HashMap<String, String>,
}

impl LoggingConfig {
    fn load_or_panic() -> LoggingConfig {
        from_env_or_panic("")
    }

    fn init_logging(self) -> tokio::task::JoinHandle<()> {
        let env_filter = tracing_subscriber::EnvFilter::from_env("TG_BOT_LOG");

        let fmt = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(std::env::var("COLORS").as_deref() != Ok("0"))
            .pretty();

        let additional_labels = GLOBAL_LABELS
            .into_iter()
            .chain(&[("source", "snowpity-tg")]);

        let mut labels = self.tg_bot_log_labels;
        labels.extend(additional_labels.map(|(k, v)| ((*k).to_owned(), (*v).to_owned())));

        let (loki, task) = tracing_loki::layer(self.loki_url, labels, HashMap::new()).unwrap();

        let join_handle = tokio::spawn(task);

        tracing_subscriber::registry()
            .with(fmt)
            .with(loki)
            .with(env_filter)
            .with(tracing_error::ErrorLayer::default())
            .init();

        join_handle
    }
}
