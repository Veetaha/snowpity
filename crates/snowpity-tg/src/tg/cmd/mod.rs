pub(crate) mod maintainer;
pub(crate) mod owner;
pub(crate) mod regular;

use crate::prelude::*;
use crate::util::DynResult;
use crate::{tg, Result};
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::fmt;
use std::sync::Arc;
use teloxide::macros::BotCommands;
use teloxide::types::{Message, User};
use teloxide::utils::markdown;

pub(crate) use maintainer::DescribeCommandError;
pub(crate) use regular::FtaiCommandError;

#[async_trait]
pub(crate) trait Command: fmt::Debug + Send + Sync + 'static {
    async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> Result;
}

pub(crate) fn handle<'a, C: Command>(
) -> impl Fn(Arc<tg::Ctx>, Message, C) -> BoxFuture<'a, DynResult> {
    move |ctx, msg, cmd| {
        let info = info_span!(
            "handle_message",
            sender = msg.from.as_ref().map(User::debug_id).as_deref(),
            // TODO: Project only text() and sender info to reduce verbosity
            msg_text = msg.text(),
            chat = %msg.chat.debug_id(),
            cmd = format_args!("{cmd:#?}")
        );

        let fut = async move {
            debug!("Processing command");

            let result = cmd.handle(&ctx, &msg).await;
            if let Err(err) = &result {
                let span = warn_span!("err", err = tracing_err(err), id = err.id());
                async {
                    if !err.is_user_error() {
                        warn!("Command handler returned an error");
                    }

                    let reply_msg = markdown::code_block(&err.display_chain().to_string());

                    let msg_result = ctx.bot.reply_chunked(&msg, reply_msg).await;

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
        };

        Box::pin(fut.instrument(info))
    }
}

/// Special case for the `/start` command in PM with the bot.
///
/// We don't want this command to appear in the help message, so we handle
/// it separately
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case")]
pub(crate) enum StartCommand {
    #[command(description = "unreachable")]
    Start,
}

#[async_trait]
impl Command for StartCommand {
    async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> Result {
        regular::Cmd::Help.handle(ctx, msg).await
    }
}

pub(crate) fn filter_pm_with_bot(msg: Message) -> bool {
    msg.chat.is_private()
}
