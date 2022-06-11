//! Telegram commands root module

use teloxide::{prelude::*, utils::command::BotCommands};
use crate::Bot;

use std::error::Error;

#[derive(BotCommands, Clone)]
#[command(rename = "snake_case", description = "These commands are supported:")]
pub(crate) enum Cmd {
    #[command(description = "display this text.")]
    Help,
}

pub(crate) async fn handle(
    bot: Bot,
    message: Message,
    cmd: Cmd,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match cmd {
        Cmd::Help => {
            bot.send_message(message.chat.id, Cmd::descriptions().to_string())
                .await?
        }
    };

    Ok(())
}
