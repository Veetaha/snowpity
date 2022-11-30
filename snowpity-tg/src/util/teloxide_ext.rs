use assert_matches::assert_matches;
use easy_ext::ext;
use teloxide::payloads::setters::*;
use teloxide::prelude::*;
use teloxide::types::{Chat, Message, MessageCommon, MessageId, MessageKind, User};
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
        self.reply_chunked(msg, markdown::escape(&Cmd::descriptions().to_string()))
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
        self.md_link()
    }
}

#[ext(ChatExt)]
pub(crate) impl Chat {
    fn debug_id(&self) -> String {
        chat_debug_id_imp(self, no_escape)
    }

    fn debug_id_markdown_escaped(&self) -> String {
        chat_debug_id_imp(self, markdown::escape)
    }
}

fn no_escape(str: &str) -> String {
    str.to_owned()
}

fn chat_debug_id_imp(chat: &Chat, escape: fn(&str) -> String) -> String {
    let title = chat.title().unwrap_or("{{unknown_chat_title}}");
    let username = chat
        .username()
        .map(|name| format!("{name}, "))
        .unwrap_or_default();

    let id = chat.id;
    let title = escape(title);
    let suffix = escape(&format!("({username}{id})"));

    chat.invite_link()
        .map(|invite_link| format!("[{title}]({invite_link}) {suffix}"))
        .unwrap_or_else(|| format!("{title} {suffix}"))
}

#[ext(MessageIdExt)]
pub(crate) impl MessageId {
    /// FIXME: this is a temporary gag. Use native display impl once the following
    /// issue is closed in teloxide: https://github.com/teloxide/teloxide/issues/760
    fn to_tracing(&self) -> &dyn tracing::Value {
        &self.0
    }
}
