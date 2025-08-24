use crate::prelude::*;
use crate::tg::Bot;
use crate::util::DynResult;
use crate::Error;
use futures::prelude::*;
use std::future::IntoFuture;
use teloxide::prelude::*;
use teloxide::types::{Chat, Message};

pub(crate) fn filter_map(msg: Message) -> Option<Chat> {
    let is_automatic_forward = msg.is_automatic_forward();
    msg.sender_chat.filter(|sender_chat| {
        // Ignore messages that were send on behalf of the chat where the bot is
        // Ignore automatic forwards for chats linked to a channel
        sender_chat.id != msg.chat.id && !is_automatic_forward
    })
}

pub(crate) async fn handle(bot: Bot, msg: Message, sender_chat: Chat) -> DynResult {
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
