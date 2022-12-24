use chrono::prelude::*;
use sqlx_bat::prelude::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::Serialize;
use teloxide::types::{self as tg_api, ChatId, UserId};

#[derive(Debug, sqlx::FromRow, Serialize)]
#[sea_query::enum_def]
pub(crate) struct TgChat {
    id: ChatId,
    kind: TgChatKind,
    title: Option<String>,
    name: Option<String>,
    invite_link: Option<String>,

    updated_at: DateTime<Utc>,
    registered_at: DateTime<Utc>,

    registered_by_user_id: UserId,
    registered_by_user_name: Option<String>,
    registered_by_user_full_name: String,
    registered_by_action: TgChatAction,

    is_captcha_enabled: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct TgChatRecord {
    pub(crate) id: i64,
    pub(crate) kind: i16,
    pub(crate) title: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) invite_link: Option<String>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) registered_at: DateTime<Utc>,
    pub(crate) registered_by_user_id: i64,
    pub(crate) registered_by_user_name: Option<String>,
    pub(crate) registered_by_user_full_name: String,
    pub(crate) registered_by_action: i16,
    pub(crate) is_captcha_enabled: bool,
}

#[derive(Debug, TryFromPrimitive, IntoPrimitive, Serialize)]
#[repr(i16)]
pub(crate) enum TgChatKind {
    PublicChannel,
    PublicGroup,
    PublicSupergroup,
    Private,
}

sqlx_bat::impl_try_into_from_db_via_std!(TgChatKind, i16);

#[derive(Debug, Clone, Copy, TryFromPrimitive, IntoPrimitive, Serialize, Eq, PartialEq)]
#[repr(i16)]
pub(crate) enum TgChatAction {
    HandleBotJoinedChat,
    HandleNewChatMember,
    ChatConfigCommand,
    ToggleCaptchaCommand,
}

sqlx_bat::impl_try_into_from_db_via_std!(TgChatAction, i16);

impl TgChatKind {
    pub(super) fn from_tg_api(chat: &tg_api::Chat) -> Self {
        match &chat.kind {
            tg_api::ChatKind::Public(chat) => match chat.kind {
                tg_api::PublicChatKind::Channel(_) => Self::PublicChannel,
                tg_api::PublicChatKind::Group(_) => Self::PublicGroup,
                tg_api::PublicChatKind::Supergroup(_) => Self::PublicSupergroup,
            },
            tg_api::ChatKind::Private(_) => Self::Private,
        }
    }
}

impl sqlx_bat::DbRepresentable for TgChat {
    type DbRepr = TgChatRecord;
}

impl sqlx_bat::TryFromDb for TgChat {
    fn try_from_db(val: Self::DbRepr) -> sqlx_bat::Result<Self> {
        let TgChatRecord {
            id,
            kind,
            title,
            name,
            invite_link,
            updated_at,
            registered_at,
            registered_by_user_id,
            registered_by_user_name,
            registered_by_user_full_name,
            registered_by_action,
            is_captcha_enabled,
        } = val;

        Ok(Self {
            id: id.try_into_app()?,
            kind: kind.try_into_app()?,
            title,
            name,
            invite_link,
            updated_at,
            registered_at,
            registered_by_user_id: registered_by_user_id.try_into_app()?,
            registered_by_user_name,
            registered_by_user_full_name,
            registered_by_action: registered_by_action.try_into_app()?,
            is_captcha_enabled,
        })
    }
}

// updated_at and registered_at fields are never constructed manually, they are
// assigned by the database automatically
#[allow(dead_code)]
fn ignore_unused_enum_variant_warning() {
    let _ = TgChatIden::UpdatedAt;
    let _ = TgChatIden::RegisteredAt;
}
