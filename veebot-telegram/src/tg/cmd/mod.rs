use crate::tg::Bot;
use crate::util::{self, tracing_err, DynError, MessageKindExt, SendMessageSettersExt};
use crate::{db, err_ctx, Result, UserError};
use display_error_chain::DisplayErrorChain;
use futures::prelude::*;
use itertools::Itertools;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;
use teloxide::utils::markdown;
use tracing::{instrument, warn, warn_span, Instrument};

#[derive(BotCommands, Clone)]
#[command(rename = "snake_case", description = "These commands are supported:")]
pub(crate) enum Cmd {
    #[command(description = "display this text.")]
    Help,

    #[command(description = "\
        disallow messages that match the given pattern (use \
        <a href = \"https://docs.rs/regex/latest/regex/#syntax\">Rust regex syntax)</a>")]
    BanPattern(String),

    #[command(description = "display the list of banned patterns")]
    BannedPatterns,

    #[command(description = "remove a message pattern from the blacklist")]
    UnbanPattern(String),
}

async fn handle_imp(bot: &Bot, msg: &Message, cmd: Cmd, repo: &db::Repo) -> Result {
    match cmd {
        Cmd::Help => {
            bot.reply(&msg, Cmd::descriptions().to_string())
                .disable_web_page_preview(true)
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Cmd::BanPattern(input) => {
            let pattern =
                Regex::new(&input).map_err(err_ctx!(UserError::InvalidRegex { input }))?;

            let created_by = msg.kind.unwrap_as_common().from.as_ref().unwrap().id;

            repo.tg_chat_banned_patterns
                .create(msg.chat.id, &pattern, created_by)
                .await?;

            bot.reply(&msg, "The pattern was successfully added to blacklist")
                .await?;
        }
        Cmd::BannedPatterns => {
            let banned_patterns: Vec<_> = repo
                .tg_chat_banned_patterns
                .get_all_by_chat_id(msg.chat.id)
                .try_collect()
                .await?;

            let futs = banned_patterns
                .iter()
                .map(|pattern| pattern.created_by)
                .collect::<HashSet<_>>()
                .into_iter()
                .map(|user_id| bot.get_chat_member(msg.chat.id, user_id));

            let users = future::try_join_all(futs)
                .await?
                .into_iter()
                .map(|member| (member.user.id, member.user.full_name()))
                .collect::<HashMap<_, _>>();

            let reply_msg = format!(
                "The following patterns are banned in this chat:\n{}",
                banned_patterns.iter().format_with("\n", |pattern, f| {
                    let regex = teloxide::utils::markdown::code_inline(pattern.pattern.as_str());

                    let creator = markdown::escape(&users.get(&pattern.created_by).unwrap());

                    let creation_time_ago = util::time_ago_from_now(pattern.created_at);

                    f(&format_args!(
                        "{regex} \\(created by {creator} {creation_time_ago}\\)"
                    ))
                })
            );

            bot.reply(&msg, reply_msg).await?;
        }
        Cmd::UnbanPattern(input) => {
            let pattern =
                Regex::new(&input).map_err(err_ctx!(UserError::InvalidRegex { input }))?;

            repo.tg_chat_banned_patterns
                .delete(msg.chat.id, &pattern)
                .await?;

            let pattern = markdown::code_inline(pattern.as_str());

            let reply_msg =
                format!("The pattern {pattern} was successfully removed from blacklist");

            bot.reply(&msg, reply_msg).await?;
        }
    };

    Ok(())
}

#[instrument(skip(bot, msg, cmd, repo), fields(msg_text = msg.text()))]
pub(crate) async fn handle(
    bot: Bot,
    msg: Message,
    cmd: Cmd,
    repo: Arc<db::Repo>,
) -> Result<(), Box<DynError>> {
    let result = handle_imp(&bot, &msg, cmd, &repo).await;
    if let Err(err) = &result {
        let span = warn_span!("err", err = tracing_err(err), id = err.id.as_str());
        async {
            if !err.kind.is_user_error() {
                warn!("Command handler returned an error");
            }

            let chain = DisplayErrorChain::new(&err);

            let reply_msg = markdown::code_block(&chain.to_string());

            let msg_result = bot.reply(&msg, reply_msg).await;

            if let Err(err) = msg_result {
                warn!(
                    err = tracing_err(&err),
                    "Failed to reply with the error message to the user"
                );
            }
        }
        .instrument(span)
        .await;
    }
    result.map_err(Into::into)
}
