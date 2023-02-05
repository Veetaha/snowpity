mod api;
mod db;
mod platform;

pub(crate) use platform::*;

use crate::posting::platform::ConfigTrait;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub(crate) struct Config {
}

impl ConfigTrait for Config {
    const ENV_PREFIX: &'static str = "DEVIANT_ART_";
}
