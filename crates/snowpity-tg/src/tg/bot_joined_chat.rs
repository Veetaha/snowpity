use crate::db::TgChatQuery;
use crate::prelude::*;
use crate::util::DynResult;
use crate::Error;
use crate::{db, tg};
use futures::prelude::*;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ChatMemberKind;

fn is_member(chat_member_kind: &ChatMemberKind) -> bool {
    use ChatMemberKind::*;
    match chat_member_kind {
        Owner(_) | Administrator(_) | Member => true,
        Restricted(restricted) => restricted.is_member,
        Left | Banned(_) => false,
    }
}

pub(crate) fn filter(update: ChatMemberUpdated) -> bool {
    !is_member(&update.old_chat_member.kind) && is_member(&update.new_chat_member.kind)
}

pub(crate) async fn handle(ctx: Arc<tg::Ctx>, update: ChatMemberUpdated) -> DynResult {
    async {
        ctx.bot
            .send_message(
                update.chat.id,
                "Hello, everyone, please love mares, and especially their snowpitys\\! 🥰",
            )
            .await?;

        info!(
            chat = update.chat.debug_id(),
            from = update.from.debug_id(),
            "Joined chat"
        );

        ctx.tg_chats
            .register_chat(TgChatQuery {
                chat: &update.chat,
                requested_by: &update.from,
                action: db::TgChatAction::HandleBotJoinedChat,
            })
            .await?;

        Ok::<_, Error>(())
    }
    .err_into()
    .await
}
