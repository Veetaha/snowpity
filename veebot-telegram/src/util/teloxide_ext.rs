use assert_matches::assert_matches;
use chrono::prelude::*;
use teloxide::payloads::setters::*;
use teloxide::prelude::*;
use teloxide::types::{Message, MessageCommon, MessageKind};
use teloxide::utils::markdown;

pub(crate) trait MessageKindExt {
    fn unwrap_as_common(&self) -> &MessageCommon;
}

impl MessageKindExt for MessageKind {
    fn unwrap_as_common(&self) -> &MessageCommon {
        assert_matches!(self, MessageKind::Common(common) => common)
    }
}

pub(crate) trait SendMessageSettersExt: Requester {
    fn reply(&self, msg: &Message, text: impl Into<String>) -> Self::SendMessage {
        self.send_message(msg.chat.id, text)
            .reply_to_message_id(msg.id)
            .allow_sending_without_reply(true)
    }
}

impl<T: Requester> SendMessageSettersExt for T {}

pub(crate) fn time_ago_from_now(past_date_time: DateTime<Utc>) -> String {
    markdown::escape(&timeago::Formatter::new().convert_chrono(past_date_time, Utc::now()))
}
