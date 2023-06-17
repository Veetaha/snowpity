use crate::prelude::*;
use crate::{err, tg, Result};
use crate::util::encoding;
use async_trait::async_trait;
use futures::prelude::*;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::future::IntoFuture;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ChatMemberKind;
use teloxide::utils::command::BotCommands;
use teloxide::utils::markdown;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "snake_case",
    description = "Commands for bot maintainer only:"
)]
pub(crate) enum Cmd {
    #[command(description = "show the guide")]
    MaintainerHelp,

    #[command(description = "display version info")]
    Version,

    #[command(description = "display short system information")]
    Sys,

    #[command(description = "display all unverified users currently registered")]
    ListUnverified,

    #[command(description = "clear the unverified users map")]
    ClearUnverified,

    #[command(description = "dump detailed diagnostic data about the message that was replied to")]
    Describe,
}

#[async_trait]
impl tg::cmd::Command for Cmd {
    async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> Result {
        match self {
            Cmd::MaintainerHelp => {
                ctx.bot.reply_help_md_escaped::<Cmd>(msg).await?;
            }
            Cmd::ListUnverified => {
                let unverified = ctx.captcha.list_unverified();
                let chats: HashSet<_> = unverified.iter().map(|(chat_id, _)| *chat_id).collect();
                let chats: HashMap<_, _> = stream::iter(chats)
                    .map(|chat_id| async move {
                        let chat: Result<_> =
                            ctx.bot.get_chat(chat_id).into_future().err_into().await;

                        let chat_debug = chat
                            .map(|chat| chat.debug_id_markdown_escaped())
                            .unwrap_or_else(|err| {
                                error!("Couldn't get chat info: {err:#?}");
                                format!("{{{{unknown_chat: {chat_id}}}}}")
                            });
                        (chat_id, chat_debug)
                    })
                    .buffer_unordered(15)
                    .collect()
                    .await;

                let mut unverified = unverified
                    .iter()
                    .map(|(chat_id, user)| {
                        let chat = &chats[chat_id];
                        let user = user.debug_id();
                        format!("{user} 👉 {chat}")
                    })
                    .join("\n");

                if unverified.is_empty() {
                    unverified = "No unverified users".to_owned();
                }

                info!("Unverified users:\n{unverified}");

                ctx.bot.reply_chunked(msg, &unverified).await?;
            }
            Cmd::ClearUnverified => {
                ctx.captcha.clear_unverified();
                ctx.bot
                    .reply_chunked(msg, "Unverified users were cleared ✔️")
                    .await?;
            }
            Cmd::Sys => {
                let info = markdown::code_block(&ctx.sysinfo.to_human_readable());
                ctx.bot.reply_chunked(msg, info).await?;
            }
            Cmd::Describe => {
                let reply = msg
                    .reply_to_message()
                    .ok_or_else(|| err!(DescribeCommandError::NoReplyMessageInDescribe))?;

                let sender = if let Some(sender) = reply.from() {
                    Some(ctx.bot.get_chat_member(msg.chat.id, sender.id).await?.kind)
                } else {
                    None
                };

                info!(
                    msg_id = msg.id.to_tracing(),
                    msg = %format_args!("\n{}", encoding::to_yaml_string(reply)),
                    sender = %format_args!("\n{}", encoding::to_yaml_string(&sender)),
                    "/describe"
                );

                #[derive(serde::Serialize)]
                struct Info<'a> {
                    message: &'a Message,

                    #[serde(skip_serializing_if = "Option::is_none")]
                    sender: Option<&'a ChatMemberKind>,
                }

                let info = encoding::to_yaml_string(&Info {
                    message: reply,
                    sender: sender.as_ref(),
                });

                let info = markdown::code_block_with_lang(&info, "json");

                ctx.bot.reply_chunked(msg, info).await?;
            }
            Cmd::Version => {
                /// Generate the key-value pairs with vergen metadata
                macro_rules! vergen_meta {
                    ( $($meta_name:literal),* $(,)? ) => {
                        [$( ($meta_name, env!(concat!("VERGEN_", $meta_name))) ),*]
                    }
                }

                let meta = vergen_meta![
                    "BUILD_TIMESTAMP",
                    "GIT_BRANCH",
                    "GIT_COMMIT_TIMESTAMP",
                    "GIT_SHA",
                    "RUSTC_CHANNEL",
                    "RUSTC_COMMIT_DATE",
                    "RUSTC_COMMIT_HASH",
                    "RUSTC_HOST_TRIPLE",
                    "RUSTC_LLVM_VERSION",
                    "RUSTC_SEMVER",
                    "CARGO_TARGET_TRIPLE",
                    "CARGO_DEBUG",
                    "CARGO_OPT_LEVEL",
                ];

                let meta = [("VERSION", env!("CARGO_PKG_VERSION"))]
                    .into_iter()
                    .chain(meta);

                let max_name_len = meta.clone().map(|(name, _)| name.len()).max().unwrap();

                let metadata = meta.format_with("\n", |(name, val), f| {
                    let name = name.to_lowercase();
                    let kv = format!("{name:<max_name_len$} = {val}");
                    f(&markdown::escape(&kv))
                });

                let metadata = format!("```\n{metadata}\n```",);

                ctx.bot.reply_chunked(msg, metadata).await?;
            }
        };

        Ok(())
    }
}

pub(crate) fn filter(ctx: Arc<tg::Ctx>, msg: Message) -> bool {
    matches!(msg.from(), Some(sender) if sender.id == ctx.cfg.maintainer)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DescribeCommandError {
    #[error("No reply message in describe command")]
    NoReplyMessageInDescribe,
}
