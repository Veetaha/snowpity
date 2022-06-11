mod cmd;
#[allow(unused)]
mod derpibooru;
mod error;
pub mod util;

use serde::Deserialize;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tracing::info;

pub use crate::error::*;

type Bot = teloxide::adaptors::AutoSend<teloxide::adaptors::Throttle<teloxide::Bot>>;

#[derive(Deserialize)]
pub struct Config {
    telegram_bot_token: String,
    // derpibooru_api_key: String,
    // derpibooru_always_on_tags: HashSet<String>,
    // derpibooru_filter: String,
}

/// Run the telegram bot processing loop
pub async fn run(config: Config) -> Result<()> {
    let http_client = util::create_http_client();

    let bot: Bot = teloxide::Bot::with_client(config.telegram_bot_token, http_client)
        .throttle(Default::default())
        .auto_send();

    info!("Starting bot...");

    bot.set_my_commands(cmd::Cmd::bot_commands()).await?;

    teloxide::commands_repl(bot, cmd::handle, cmd::Cmd::ty()).await;

    info!("Bot stopped");

    Ok(())
}
