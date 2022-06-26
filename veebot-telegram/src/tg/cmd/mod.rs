pub(crate) mod maintainer;
pub(crate) mod regular;

use crate::db;
use crate::tg::Bot;
use crate::util::prelude::*;
use crate::util::{tracing_err, DynError};
use async_trait::async_trait;
use display_error_chain::DisplayErrorChain;
use futures::future::BoxFuture;
use std::sync::Arc;
use teloxide::types::Message;
use teloxide::utils::markdown;
use tracing::{info_span, warn, warn_span};
use tracing_futures::Instrument;

#[async_trait]
pub(crate) trait HandleImp<C>: Send + Sync + 'static {
    async fn handle_imp(&self, bot: &Bot, msg: &Message, repo: &db::Repo, cmd: C) -> crate::Result;
}

pub(crate) fn handle<'a, C>(
    imp: &'a dyn HandleImp<C>,
) -> impl Fn(Bot, Message, C, Arc<db::Repo>) -> BoxFuture<'a, Result<(), Box<DynError>>>
where
    C: Send + std::fmt::Debug + 'static,
{
    move |bot, msg, cmd, repo| {
        let info = info_span!(
            "handle_message",
            // TODO: Project only text() and sender info to reduce verbosity
            msg = format_args!("{msg:#?}"),
            cmd = format_args!("{cmd:#?}")
        );

        let fut = async move {
            let result = imp.handle_imp(&bot, &msg, &repo, cmd).await;
            if let Err(err) = &result {
                let span = warn_span!("err", err = tracing_err(err), id = err.id.as_str());
                async {
                    if !err.kind.is_user_error() {
                        warn!("Command handler returned an error");
                    }

                    let chain = DisplayErrorChain::new(&err);

                    let reply_msg = markdown::code_block(&chain.to_string());

                    let msg_result = bot.reply_chunked(&msg, reply_msg).await;

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
