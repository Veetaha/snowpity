pub(crate) mod derpi;
pub(crate) mod twitter;

use crate::{config, http};

pub(crate) struct Config {
    derpi: derpi::Config,
    twitter: twitter::Config,
}

impl Config {
    pub fn load_or_panic() -> Config {
        Self {
            derpi: config::from_env_or_panic("DERPI_"),
            twitter: config::from_env_or_panic("TWITTER_"),
        }
    }
}

pub(crate) struct Client {
    pub(crate) derpi: derpi::Client,
    pub(crate) twitter: twitter::Client,
}

impl Client {
    pub fn new(cfg: Config, http: http::Client) -> Self {
        Self {
            derpi: derpi::Client::new(cfg.derpi, http.clone()),
            twitter: twitter::Client::new(cfg.twitter, http),
        }
    }
}
