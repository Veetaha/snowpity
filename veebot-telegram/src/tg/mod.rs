//! Telegram commands root module

mod cmd;
mod updates;

use crate::util;
use crate::{Result, TgConfig};
use cmd::Cmd;
use dptree::di::DependencyMap;
use teloxide::adaptors::{AutoSend, CacheMe, DefaultParseMode, Throttle, Trace};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;
use tracing::info;

type Bot = AutoSend<Trace<CacheMe<DefaultParseMode<Throttle<teloxide::Bot>>>>>;

pub(crate) async fn run_bot(di: DependencyMap, config: TgConfig) -> Result {
    let http_client = util::create_http_client();

    let bot: Bot = teloxide::Bot::with_client(config.bot_token, http_client)
        .throttle(Default::default())
        .parse_mode(ParseMode::MarkdownV2)
        .cache_me()
        .trace(teloxide::adaptors::trace::Settings::all())
        .auto_send();

    info!("Starting bot...");

    bot.set_my_commands(Cmd::bot_commands()).await?;

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Cmd>()
                .endpoint(cmd::handle),
        )
        .branch(Update::filter_message().endpoint(updates::handle_message))
        // .branch(Update::filter_edited_message().endpoint(updates::handle_edited_message))
        .branch(Update::filter_my_chat_member().endpoint(updates::handle_my_chat_member));

    Dispatcher::builder(bot, handler)
        .dependencies(di)
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;

    info!("Bot stopped");

    Ok(())
}
