use assert_matches::assert_matches;
use easy_ext::ext;
use teloxide::payloads::setters::*;
use teloxide::prelude::*;
use teloxide::types::{Chat, Message, MessageCommon, MessageKind, User};
use teloxide::utils::markdown;

#[ext(MessageKindExt)]
pub(crate) impl MessageKind {
    fn unwrap_as_common(&self) -> &MessageCommon {
        assert_matches!(self, MessageKind::Common(common) => common)
    }
}

/// There is [`RequesterExt`] in [`teloxide::prelude`]. We name this symbol
/// different to avoid collisions.
#[ext(UtilRequesterExt)]
pub(crate) impl<T> T
where
    Self: Requester,
{
    /// Send a message to the chat, but split it into multiple ones if it's too long.
    fn reply_chunked(&self, msg: &Message, text: impl Into<String>) -> Self::SendMessage {
        self.send_message(msg.chat.id, text)
            .reply_to_message_id(msg.id)
            .allow_sending_without_reply(true)
    }

    fn reply_help_md_escaped<Cmd: teloxide::utils::command::BotCommands>(
        &self,
        msg: &Message,
    ) -> Self::SendMessage {
        self.reply_chunked(&msg, markdown::escape(&Cmd::descriptions().to_string()))
    }
}

#[ext(UserExt)]
pub(crate) impl User {
    fn md_link(&self) -> String {
        let mention_text =
            markdown::escape(&self.username.clone().unwrap_or_else(|| self.full_name()));
        format!("[{mention_text}]({})", self.url())
    }

    fn debug_id(&self) -> String {
        let full_name = self.full_name();
        let id = self.id;
        format!("{full_name} ({id})")
    }
}

#[ext(ChatExt)]
pub(crate) impl Chat {
    fn debug_id(&self) -> String {
        let username = self.username().unwrap_or("{{unknown_chat_username}}");
        let id = self.id;
        format!("{username} ({id})")
    }
}
