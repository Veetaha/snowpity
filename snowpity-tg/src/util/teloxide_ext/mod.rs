mod requester;

use easy_ext::ext;
use teloxide::types::{Chat, MessageId, UpdateKind, User};
use teloxide::utils::markdown;

pub(crate) mod prelude {
    pub(crate) use super::{
        requester::UtilRequesterExt as _, ChatExt as _, MessageIdExt as _, UpdateKindExt as _,
        UserExt as _,
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
        // because links to of this form to users with the restricted
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
        .map(markdown::escape_link_url)
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
            InlineQuery
            ChosenInlineResult
            CallbackQuery
            ShippingQuery
            PreCheckoutQuery
            Poll
            PollAnswer
            MyChatMember
            ChatMember
            ChatJoinRequest
            Error
        }
    }
}
