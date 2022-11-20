use serde::Deserialize;
use teloxide::types::{UserId, ChatId};


#[derive(Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) bot_token: String,

    /// ID of the user, who owns the bot, and thus has full access to it
    pub(crate) bot_maintainer: UserId,

    pub(crate) media_cache_chat_id: ChatId,
}
