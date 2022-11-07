pub(crate) mod admin;
pub(crate) mod maintainer;
pub(crate) mod regular;
use crate::tg;
use crate::util::prelude::*;
use crate::util::{tracing_err, DynError};
use async_trait::async_trait;
use display_error_chain::DisplayErrorChain;
use futures::future::BoxFuture;
use std::fmt;
use std::sync::Arc;
use teloxide::types::{Message, User};
use teloxide::utils::markdown;
use tracing::{debug, info_span, warn, warn_span};
use tracing_futures::Instrument;

#[async_trait]
pub(crate) trait Command: fmt::Debug + Send + Sync + 'static {
    async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> crate::Result;
}

pub(crate) fn handle<'a, C: Command>(
) -> impl Fn(Arc<tg::Ctx>, Message, C) -> BoxFuture<'a, Result<(), Box<DynError>>> {
    move |ctx, msg, cmd| {
        let info = info_span!(
            "handle_message",
            sender = msg.from().map(User::debug_id).as_deref(),
            // TODO: Project only text() and sender info to reduce verbosity
            msg_text = msg.text(),
            chat = msg.chat.debug_id().as_str(),
            cmd = format_args!("{cmd:#?}")
        );

        let fut = async move {
            debug!("Processing command");

            let result = cmd.handle(&ctx, &msg).await;
            if let Err(err) = &result {
                let span = warn_span!("err", err = tracing_err(err), id = err.id.as_str());
                async {
                    if !err.kind.is_user_error() {
                        warn!("Command handler returned an error");
                    }

                    let chain = DisplayErrorChain::new(&err);

                    let reply_msg = markdown::code_block(&chain.to_string());

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
