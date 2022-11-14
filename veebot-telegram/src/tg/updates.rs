#![allow(unused)]
// FIXME: remove this ^
// FIXME: remove this ^
// FIXME: remove this ^
// FIXME: remove this ^
// FIXME: remove this ^
// FIXME: remove this ^
// FIXME: remove this ^

use crate::tg::{Bot, Ctx};
use crate::util::DynError;
use crate::DynResult;
use crate::Error;
use crate::Result;
use futures::prelude::*;
use std::future::IntoFuture;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::Chat;
use teloxide::types::{ChatMemberUpdated, Message};
// use teloxide::utils::markdown;
use tracing::info;
use tracing::instrument;

enum PhrasesFilterRequest {
    ValidateMessage {
        msg: Message,
        // response: tokio::sync::oneshot::Receiver<censy::ValidationOutput>,
    },
    ListBannedPhrases {
        // response: tokio::sync::oneshot::Receiver<Vec<censy::TemplatePhrase>>,
    },
}

struct PhrasesFilterService {
    db: Arc<crate::db::Repo>,
}

impl PhrasesFilterService {
    fn phrases_filter_service(
        requests: impl Stream<Item = PhrasesFilterRequest> + Unpin + Send + 'static,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async {
            // loop {
            //     let request = match requests.next().await {
            //         Some(request) => {}
            //         None => {
            //             info!("Phrases filter service is shutting down since the requests stream has ended");
            //             break;
            //         }
            //     };
            // }
        })
    }
}

#[instrument(skip(ctx, msg), fields(msg_text = msg.text()))]
pub(crate) async fn handle_message(ctx: Arc<Ctx>, msg: Message) -> DynResult {
    async {
        // let text = match msg.text() {
        //     Some(text) => text,
        //     None => return Ok(()),
        // };

        // let banned_pattern = db
        //     .tg_chat_banned_words
        //     .get_all_by_chat_id(msg.chat.id)
        //     .try_collect::<Vec<_>>()
        //     .await?
        //     .into_iter()
        //     .find(|pattern| pattern.pattern.is_match(text));

        // let pattern = match banned_pattern {
        //     Some(pattern) => pattern,
        //     None => return Ok(()),
        // };

        // let chat = db.tg_chats.get_by_id(msg.chat.id).await?;

        // // Reply with a message to warn the user
        // {
        //     let pattern = markdown::code_inline(pattern.pattern.as_str());
        //     let reply_msg = format!("The pattern {pattern} was banned in this chat");

        //     bot.reply_chunked(&msg, reply_msg).await?;
        // }

        // bot.restrict_chat_member()

        Ok::<_, Error>(())
    }
    .err_into()
    .await
}

pub(crate) fn filter_message_from_channel(msg: Message) -> Option<Chat> {
    msg.sender_chat().cloned().filter(|sender_chat| {
        // Ignore messages that were send on behalf of the chat where the bot is
        // Ignore automatic forwards for chats linked to a channel
        sender_chat.id != msg.chat.id && !msg.is_automatic_forward()
    })
}

pub(crate) async fn handle_message_from_channel(
    bot: Bot,
    msg: Message,
    sender_chat: Chat,
) -> DynResult {
    async {
        info!(
            sender_chat = format_args!("{sender_chat:#?}"),
            "Found a message from ambient channel. Removing it and banning sender chat...",
        );

        futures::try_join!(
            bot.delete_message(msg.chat.id, msg.id).into_future(),
            bot.ban_chat_sender_chat(msg.chat.id, sender_chat.id)
                .into_future(),
        )?;

        Ok::<_, Error>(())
    }
    .err_into()
    .await
}
