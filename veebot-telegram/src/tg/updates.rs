use crate::tg::Bot;
use crate::util::prelude::*;
use crate::util::DynError;
use crate::Result;
use crate::{db, Error};
use futures::prelude::*;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatMemberUpdated, Message};
use teloxide::utils::markdown;

pub(crate) async fn handle_my_chat_member(
    bot: Bot,
    upd: ChatMemberUpdated,
) -> Result<(), Box<DynError>> {
    bot.send_message(
        upd.chat.id,
        markdown::escape("Hello everyone! I am going to be your overmare."),
    )
    .await?;

    Ok(())
}

pub(crate) async fn handle_message(
    bot: Bot,
    msg: Message,
    db: Arc<db::Repo>,
) -> Result<(), Box<DynError>> {
    async {
        let text = match msg.text() {
            Some(text) => text,
            None => return Ok(()),
        };

        let banned_pattern = db
            .tg_chat_banned_patterns
            .get_all_by_chat_id(msg.chat.id)
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .find(|pattern| pattern.pattern.is_match(text));

        let pattern = match banned_pattern {
            Some(pattern) => pattern,
            None => return Ok(()),
        };

        let chat = db.tg_chats.get_by_id(msg.chat.id).await?;

        // Reply with a message to warn the user
        {
            let pattern = markdown::code_inline(pattern.pattern.as_str());
            let reply_msg = format!("The pattern {pattern} was banned in this chat");
            bot.reply(&msg, reply_msg).await?;
        }

        // bot.restrict_chat_member()

        Ok::<_, Error>(())
    }
    .await
    .map_err(Into::into)
}
