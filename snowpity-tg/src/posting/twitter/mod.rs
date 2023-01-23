mod api;
mod db;
mod platform;

pub(crate) use api::TwitterError;
pub(crate) use platform::*;

use crate::posting::platform::ConfigTrait;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub(crate) struct Config {
    bearer_token: String,
}

impl ConfigTrait for Config {
    const ENV_PREFIX: &'static str = "TWITTER_";
}
