//! Rust analyzer is very slow on processing requester extension here, so extracted
//! it to a separate module to limit the scope of analysis.

use teloxide::requests::Requester;
use teloxide::types::Message;
use teloxide::utils::markdown;
use teloxide::prelude::*;
use easy_ext::ext;

/// There is [`RequesterExt`] in [`teloxide::prelude`]. We name this symbol
/// different to avoid collisions.
#[ext(UtilRequesterExt)]
pub(crate) impl<T: Requester> T {
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
