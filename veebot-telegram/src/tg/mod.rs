//! Telegram commands root module

mod captcha;
mod cmd;
mod updates;

use std::sync::Arc;

use crate::ftai::FtaiService;
use crate::util;
use crate::{Result, TgConfig};
use dptree::di::DependencyMap;
use teloxide::adaptors::{AutoSend, CacheMe, DefaultParseMode, Throttle, Trace};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;
use tracing::info;

type Bot = AutoSend<Trace<CacheMe<DefaultParseMode<Throttle<teloxide::Bot>>>>>;

pub(crate) struct Ctx {
    bot: Bot,
    // db: db::Repo,
    cfg: TgConfig,
    ftai: FtaiService,
}

pub(crate) async fn run_bot(cfg: TgConfig) -> Result {
    let mut di = DependencyMap::new();

    let http = util::create_http_client();

    let bot: Bot = teloxide::Bot::with_client(cfg.bot_token.clone(), http.clone())
        .throttle(Default::default())
        .parse_mode(ParseMode::MarkdownV2)
        .cache_me()
        .trace(teloxide::adaptors::trace::Settings::all())
        .auto_send();

    let ftai = FtaiService::new(http);

    di.insert(Arc::new(Ctx {
        bot: bot.clone(),
        // db,
        cfg,
        ftai,
    }));

    info!("Starting bot...");

    bot.set_my_commands(cmd::regular::Cmd::bot_commands())
        .await?;

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .chain(Message::filter_new_chat_members())
                .endpoint(captcha::handle_new_chat_members),
        )
        .branch(
            Update::filter_message()
                .chain(Message::filter_left_chat_member())
                .endpoint(captcha::handle_left_chat_member),
        )
        .branch(
            Update::filter_message()
                .chain(dptree::filter_map(updates::filter_message_from_channel))
                .endpoint(updates::handle_message_from_channel),
        )
        .branch(
            Update::filter_message()
                .filter_command::<cmd::regular::Cmd>()
                .endpoint(cmd::handle::<cmd::regular::Cmd>()),
        )
        .branch(
            Update::filter_message()
                .filter_command::<cmd::maintainer::Cmd>()
                .chain(dptree::filter(cmd::maintainer::is_maintainer))
                .endpoint(cmd::handle::<cmd::maintainer::Cmd>()),
        )
        // .branch(Update::filter_edited_message().endpoint(updates::handle_edited_message))
        .branch(Update::filter_my_chat_member().endpoint(updates::handle_my_chat_member))
        .branch(Update::filter_callback_query().endpoint(captcha::handle_callback_query));

    Dispatcher::builder(bot, handler)
        .dependencies(di)
        // We don't handle all possible messages that users send,
        // so to supress the warning that we don't do this we have
        // a noop default handler here
        .default_handler(|_| std::future::ready(()))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;

    info!("Bot stopped");

    Ok(())
}
