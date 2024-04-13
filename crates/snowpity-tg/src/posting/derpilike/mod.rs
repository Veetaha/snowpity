mod api;
mod db;
mod platform;

mod platform_2;
pub(crate) mod platform_3;

pub(crate) use platform::*;

use crate::posting::platform::ConfigTrait;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub(crate) struct Config {
    // Derpibooru doesn't require an API key for read-only requests.
    // The rate limiting is also the same for both anonymous and authenticated requests,
    // therefore we don't really need an API key
    //
    // This was confirmed by the Derpibooru staff in discord:
    // https://discord.com/channels/430829008402251796/438029140659142657/1059492359122989146
    //
    // This config struct exists here, just in case some day we do need to use an API key,
    // or want any other config options.
    //
    // api_key: String,
}

impl ConfigTrait for Config {
    const ENV_PREFIX: &'static str = "DERPIBOORU_";
}
