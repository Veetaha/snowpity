//! Telegram commands root module

mod bot_joined_chat;
mod captcha;
mod cmd;
mod config;
mod inline_query;
mod media_cache;
mod message_from_channel;

use crate::ftai::FtaiService;
use crate::prelude::*;
use crate::sysinfo::SysInfoService;
use crate::{db, encoding, http, media_host, Result};
use captcha::CaptchaCtx;
use dptree::di::DependencyMap;
use inline_query::InlineQueryService;
use std::sync::Arc;
use teloxide::adaptors::{CacheMe, DefaultParseMode, Throttle, Trace};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;

pub(crate) use cmd::{DescribeCommandError, FtaiCommandError};
pub(crate) use config::*;
pub(crate) use media_cache::{MediaCacheError, TgFileMeta};

pub(crate) type Bot = Trace<CacheMe<DefaultParseMode<Throttle<teloxide::Bot>>>>;

metrics_bat::labels! {
    TgUpdateLabels { kind }
}

metrics_bat::counters! {
    /// Number of updates received from Telegram
    tg_updates_total;

    /// Number of updates received from Telegram, that were skipped by the bot
    tg_updates_skipped_total;
}

pub(crate) struct Ctx {
    bot: Bot,
    db: Arc<db::Repo>,
    cfg: Arc<Config>,
    ftai: FtaiService,
    captcha: CaptchaCtx,
    sysinfo: SysInfoService,
    inline_query: InlineQueryService,
}

pub(crate) struct RunBotOptions {
    pub(crate) tg_cfg: Config,
    pub(crate) db: db::Repo,
    pub(crate) media_cfg: media_host::Config,
}

pub(crate) async fn run_bot(opts: RunBotOptions) -> Result {
    let mut di = DependencyMap::new();

    let http = http::create_client();

    let bot: Bot = teloxide::Bot::new(opts.tg_cfg.token.clone())
        .throttle(Default::default())
        .parse_mode(ParseMode::MarkdownV2)
        .cache_me()
        .trace(teloxide::adaptors::trace::Settings::all());

    let ftai = FtaiService::new(http.clone());
    let media = Arc::new(media_host::Client::new(opts.media_cfg, http.clone()));
    let tg_cfg = Arc::new(opts.tg_cfg);
    let db = Arc::new(opts.db);

    let ctx = media_cache::Context {
        http,
        bot: bot.clone(),
        media,
        cfg: tg_cfg.clone(),
        db: db.clone(),
    };

    di.insert(Arc::new(Ctx {
        db,
        bot: bot.clone(),
        cfg: tg_cfg,
        ftai,
        sysinfo: SysInfoService::new(),
        captcha: Default::default(),
        inline_query: InlineQueryService::new(ctx),
    }));

    info!("Starting bot...");

    bot.set_my_commands(cmd::regular::Cmd::bot_commands())
        .await?;

    let handler = dptree::entry()
        .inspect(|update: Update| {
            let labels = TgUpdateLabels {
                kind: update.kind.discriminator(),
            };
            tg_updates_total(labels).increment(1);
            trace!(
                target: "tg_update",
                "{}",
                encoding::to_json_string_pretty(&update),
            );
        })
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
                .chain(dptree::filter_map(message_from_channel::filter))
                .endpoint(message_from_channel::handle),
        )
        .branch(
            Update::filter_message()
                .filter_command::<cmd::regular::Cmd>()
                .endpoint(cmd::handle::<cmd::regular::Cmd>()),
        )
        .branch(
            Update::filter_message()
                .filter_command::<cmd::owner::Cmd>()
                .chain(dptree::filter_async(cmd::owner::filter))
                .endpoint(cmd::handle::<cmd::owner::Cmd>()),
        )
        .branch(
            Update::filter_message()
                .filter_command::<cmd::maintainer::Cmd>()
                .chain(dptree::filter(cmd::maintainer::filter))
                .endpoint(cmd::handle::<cmd::maintainer::Cmd>()),
        )
        .branch(Update::filter_callback_query().endpoint(captcha::handle_callback_query))
        .branch(Update::filter_inline_query().endpoint(inline_query::handle))
        .branch(
            Update::filter_chosen_inline_result()
                .endpoint(inline_query::handle_chosen_inline_result),
        )
        .branch(
            Update::filter_my_chat_member()
                .filter(bot_joined_chat::filter)
                .endpoint(bot_joined_chat::handle),
        )
        .inspect(|update: Update| {
            let labels = TgUpdateLabels {
                kind: update.kind.discriminator(),
            };
            tg_updates_skipped_total(labels).increment(1)
        });

    Dispatcher::builder(bot, handler)
        .dependencies(di)
        // We don't handle all possible messages that users send,
        // so to suppress the warning that we don't do this we have
        // a noop default handler here
        .default_handler(|_| std::future::ready(()))
        // TODO: better log the error
        // .error_handler(handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Bot stopped");

    Ok(())
}
