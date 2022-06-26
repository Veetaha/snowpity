use crate::tg::Bot;
use crate::util::DynError;
use crate::Error;
use crate::Result;
use futures::prelude::*;
use teloxide::prelude::*;
use teloxide::types::Chat;
use teloxide::types::{ChatMemberUpdated, Message};
use teloxide::utils::markdown;
use tracing::info;

pub(crate) async fn handle_my_chat_member(
    bot: Bot,
    upd: ChatMemberUpdated,
) -> Result<(), Box<DynError>> {
    // TODO: Send this message only when bot is invited to a chat
    bot.send_message(
        upd.chat.id,
        markdown::escape("Hello everyone! I am going to be your overmare."),
    )
    .await?;

    Ok(())
}

// #[instrument(skip(bot, msg, _db), fields(msg_text = msg.text()))]
// pub(crate) async fn handle_message(
//     bot: Bot,
//     msg: Message,
//     _db: Arc<db::Repo>,
// ) -> Result<(), Box<DynError>> {
//     async {
//         match &msg.kind {
//             MessageKind::NewChatMembers(members) => {
//                 return captcha::handle_new_chat_members(bot, &msg, members).await;
//             }

//             MessageKind::LeftChatMember(member) => {
//                 return captcha::handle_left_chat_member(bot, &msg, member).await;
//             }

//             _ => {}
//         }

//         // TODO: handling of banned patterns here:

//         // let text = match msg.text() {
//         //     Some(text) => text,
//         //     None => return Ok(()),
//         // };

//         // let banned_pattern = db
//         //     .tg_chat_banned_patterns
//         //     .get_all_by_chat_id(msg.chat.id)
//         //     .try_collect::<Vec<_>>()
//         //     .await?
//         //     .into_iter()
//         //     .find(|pattern| pattern.pattern.is_match(text));

//         // let pattern = match banned_pattern {
//         //     Some(pattern) => pattern,
//         //     None => return Ok(()),
//         // };

//         // let chat = db.tg_chats.get_by_id(msg.chat.id).await?;

//         // // Reply with a message to warn the user
//         // {
//         //     let pattern = markdown::code_inline(pattern.pattern.as_str());
//         //     let reply_msg = format!("The pattern {pattern} was banned in this chat");

//         //     bot.reply_chunked(&msg, reply_msg).await?;
//         // }

//         // // bot.restrict_chat_member()

//         Ok::<_, Error>(())
//     }
//     .err_into()
//     .await
// }

pub(crate) fn filter_message_from_channel<'m>(msg: Message) -> Option<Chat> {
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
) -> Result<(), Box<DynError>> {
    async {
        info!(
            sender_chat = format_args!("{sender_chat:#?}"),
            "Found a message from ambient channel. Removing it and banning sender chat...",
        );

        futures::try_join!(
            bot.delete_message(msg.chat.id, msg.id),
            bot.ban_chat_sender_chat(msg.chat.id, sender_chat.id),
        )?;

        Ok::<_, Error>(())
    }
    .err_into()
    .await
}
