//! Telegram commands root module

mod captcha;
mod cmd;
mod config;
mod inline_query;
mod updates;

use crate::db;
use crate::derpi::{self, DerpiService};
use crate::ftai::FtaiService;
use crate::metrics::def_metrics;
use crate::sysinfo::SysInfoService;
use crate::util::prelude::*;
use crate::util::{self, encoding};
use crate::Result;
use captcha::CaptchaCtx;
use dptree::di::DependencyMap;
use inline_query::InlineQueryService;
use std::sync::Arc;
use teloxide::adaptors::throttle::ThrottlingRequest;
use teloxide::adaptors::trace::TraceRequest;
use teloxide::adaptors::{CacheMe, DefaultParseMode, Throttle, Trace};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::requests::MultipartRequest;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;

use crate::tg::inline_query::media_cache;
pub(crate) use config::*;

pub(crate) type Bot = Trace<CacheMe<DefaultParseMode<Throttle<teloxide::Bot>>>>;
pub(crate) type Request<T> = TraceRequest<ThrottlingRequest<MultipartRequest<T>>>;

def_metrics! {
    /// Number of updates received from telegram
    tg_updates: IntCounter;

    /// Number of updates received from telegram, that were skipped by the bot
    tg_updates_skipped: IntCounter;
}

pub(crate) struct Ctx {
    bot: Bot,
    cfg: Arc<Config>,
    ftai: FtaiService,
    captcha: CaptchaCtx,
    sysinfo: SysInfoService,
    inline_query: InlineQueryService,
}

pub(crate) async fn run_bot(tg_cfg: Config, derpi_cfg: derpi::Config, db: db::Repo) -> Result {
    let mut di = DependencyMap::new();

    let http = util::http::create_client();

    let bot: Bot = teloxide::Bot::new(tg_cfg.token.clone())
        .throttle(Default::default())
        .parse_mode(ParseMode::MarkdownV2)
        .cache_me()
        .trace(teloxide::adaptors::trace::Settings::all());

    let ftai = FtaiService::new(http.clone());
    let derpi = Arc::new(DerpiService::new(derpi_cfg, http.clone()));
    let tg_cfg = Arc::new(tg_cfg);
    let db = Arc::new(db);

    let ctx = media_cache::Context {
        http_client: http,
        bot: bot.clone(),
        derpi,
        cfg: tg_cfg.clone(),
        db,
    };

    di.insert(Arc::new(Ctx {
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
            tg_updates().inc();
            trace!(
                target: "tg_updates",
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
        .branch(Update::filter_callback_query().endpoint(captcha::handle_callback_query))
        .branch(Update::filter_inline_query().endpoint(inline_query::handle_inline_query))
        .branch(
            Update::filter_chosen_inline_result()
                .endpoint(inline_query::handle_chosen_inline_result),
        )
        .inspect(|| tg_updates_skipped().inc());

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
