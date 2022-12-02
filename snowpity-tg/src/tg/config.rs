use serde::Deserialize;
use teloxide::types::{ChatId, UserId};

#[derive(Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) token: String,

    /// ID of the user, who owns the bot, and thus has full access to it
    pub(crate) maintainer: UserId,

    pub(crate) media_cache_chat: ChatId,
}
