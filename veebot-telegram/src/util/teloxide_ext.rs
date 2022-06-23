use assert_matches::assert_matches;
use chrono::prelude::*;
use easy_ext::ext;
use teloxide::payloads::setters::*;
use teloxide::prelude::*;
use teloxide::types::{Message, MessageCommon, MessageKind, User};
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
    Self: Requester
{
    /// Send a message to the chat, but split it into multiple ones if it's too long.
    fn reply_chunked(&self, msg: &Message, text: impl Into<String>) -> Self::SendMessage {
        self.send_message(msg.chat.id, text)
            .reply_to_message_id(msg.id)
            .allow_sending_without_reply(true)
    }
}

#[ext(UserExt)]
pub(crate) impl User {
    fn md_link(&self) -> String {
        let mention_text =
            markdown::escape(&self.username.clone().unwrap_or_else(|| self.full_name()));
        format!("[{mention_text}]({})", self.preferably_tme_url())
    }
}

// #[ext(SendMessageRequestExt)]
// #[async_trait]
// pub(crate) impl<T: Sized + Request<Payload = SendMessage>> T {
//     async fn send_message_cunked(mut self, chat_id: impl Into<Recipient>, text: impl Into<String>) -> Result<Message>;
// }

pub(crate) fn time_ago_from_now(past_date_time: DateTime<Utc>) -> String {
    markdown::escape(&timeago::Formatter::new().convert_chrono(past_date_time, Utc::now()))
}
