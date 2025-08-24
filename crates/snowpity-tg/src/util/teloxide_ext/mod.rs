mod requester;

use easy_ext::ext;
use teloxide::types::{Chat, ChatFullInfo, ChatId, MessageId, UpdateKind, User};
use teloxide::utils::markdown;

pub(crate) mod prelude {
    pub(crate) use super::{
        requester::UtilRequesterExt as _, ChatExt as _, ChatFullInfoExt as _, MessageIdExt as _,
        UpdateKindExt as _, UserExt as _,
    };
}

#[ext(UserExt)]
pub(crate) impl User {
    fn username(&self) -> String {
        self.username.clone().unwrap_or_else(|| self.full_name())
    }

    fn md_link(&self) -> String {
        let mention_text = markdown::escape(&self.full_name());

        // We are using `preferably_tme_url` instead of user ID style `tg://user?id={}`
        // because links of this form to users with the restricted
        // 'Forwarded Messages' privacy setting won't be clickable in telegram
        // messages.
        markdown::link(self.preferably_tme_url().as_str(), &mention_text)
    }

    fn debug_id(&self) -> String {
        format!("{} ({})", self.username(), self.id)
    }
}

#[ext(ChatExt)]
pub(crate) impl Chat {
    fn debug_id(&self) -> String {
        chat_debug_id_imp(self.id, self.title(), self.username(), no_escape)
    }
}

#[ext(ChatFullInfoExt)]
pub(crate) impl ChatFullInfo {
    fn debug_id_markdown_escaped(&self) -> String {
        chat_debug_id_imp(self.id, self.title(), self.username(), markdown::escape)
    }
}

fn no_escape(str: &str) -> String {
    str.to_owned()
}

fn chat_debug_id_imp(
    chat_id: ChatId,
    chat_title: Option<&str>,
    chat_username: Option<&str>,
    escape: fn(&str) -> String,
) -> String {
    let title = chat_title.unwrap_or("{{unknown_chat_title}}");
    let username = chat_username
        .map(|name| format!("{name}, "))
        .unwrap_or_default();

    let title = escape(title);
    let suffix = escape(&format!("({username}{chat_id})"));

    format!("{title} {suffix}")
}

#[ext(MessageIdExt)]
pub(crate) impl MessageId {
    /// FIXME: this is a temporary gag. Use native display impl once the following
    /// issue is closed in teloxide: https://github.com/teloxide/teloxide/issues/760
    fn to_tracing(&self) -> &dyn tracing::Value {
        &self.0
    }
}

#[ext(UpdateKindExt)]
pub(crate) impl UpdateKind {
    fn discriminator(&self) -> &'static str {
        macro_rules! stringify_enum {
            ($val:expr, $($variant:ident)*) => {
                match $val {$( UpdateKind::$variant(_) => stringify!($variant), )*}
            }
        }
        stringify_enum! {
            self,
            Message
            EditedMessage
            ChannelPost
            EditedChannelPost
            BusinessConnection
            BusinessMessage
            EditedBusinessMessage
            DeletedBusinessMessages
            MessageReaction
            MessageReactionCount
            InlineQuery
            ChosenInlineResult
            CallbackQuery
            ShippingQuery
            PreCheckoutQuery
            PurchasedPaidMedia
            Poll
            PollAnswer
            MyChatMember
            ChatMember
            ChatJoinRequest
            ChatBoost
            RemovedChatBoost
            Error
        }
    }
}
