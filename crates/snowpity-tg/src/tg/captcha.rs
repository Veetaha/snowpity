use crate::prelude::*;
use crate::tg::{self, Bot};
use crate::util::{encoding, DynResult};
use crate::{db, err, Error, ErrorKind, Result};
use chrono::prelude::*;
use futures::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex as SyncMutex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::IntoFuture;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::{
    ChatMember, ChatMemberKind, ChatPermissions, InlineKeyboardButton, InputFile, Message,
    MessageId, ReplyMarkup, ReplyParameters, Restricted, UntilDate, User,
};
use teloxide::utils::markdown;
use tokio::sync::oneshot;

/// Duration for the new users to solve the captcha. If they don't reply
/// in this time, they will be kicked.
const CAPTCHA_VERIFICATION_TIMEOUT: Duration = Duration::from_secs(3 * 60);

/// Duration that is added to [`CAPTCHA_VERIFICATION_TIMEOUT`] to guarantee that
/// the restrictions for the user will be lifted after the timeout of the bot
/// doesn't lift them earlier.
const CAPTCHA_RESTRICTIONS_TIMEOUT: Duration = Duration::from_secs(60);

/// Duration for the ban of the users that didn't pass captcha.
const CAPTCHA_BAN_DURATION: Duration = Duration::from_secs(2 * 60);

const GREETING_ANIMATION_URL: &str = "https://derpicdn.net/img/2021/12/19/2767482/small.gif";

#[derive(Default)]
pub(crate) struct CaptchaCtx {
    /// Map of users that are in the processes of solving captcha.
    unverified_users: SyncMutex<HashMap<(ChatId, UserId), UnverifiedUser>>,
}

#[derive(Debug)]
pub(crate) struct UnverifiedUser {
    member: ChatMember,
    chat_id: ChatId,
    captcha_msg_id: MessageId,
    captcha_timeout_cancel: Option<oneshot::Sender<()>>,
    restricted_until_date: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CaptchaReplyPayload {
    expected_user_id: UserId,
    allowed: bool,
}

#[instrument(skip_all, fields(
    from = %callback_query.from.debug_id(),
    chat = callback_query.message.as_ref()
        .map(|msg| msg.chat().debug_id())
        .as_deref()
        .unwrap_or("{{unknown_chat}}"),
))]
pub(crate) async fn handle_callback_query(
    ctx: Arc<tg::Ctx>,
    callback_query: CallbackQuery,
) -> DynResult {
    async {
        let tg::Ctx { bot, .. } = &*ctx;

        debug!("Processing callback query");

        let Some(callback_data) = callback_query.data else {
            warn!("Received empty callback data");
            return Ok(());
        };

        let Some(captcha_msg) = callback_query.message else {
            warn!("Received empty callback message");
            return Ok(());
        };

        let payload: CaptchaReplyPayload = encoding::secure_decode(&callback_data)?;

        let user_id = callback_query.from.id;
        let chat_id = captcha_msg.chat().id;

        let payload_span = {
            let expected_user = ctx
                .captcha
                .unverified_users
                .lock()
                .values()
                .find(|unverified| {
                    unverified.captcha_msg_id == captcha_msg.id() && unverified.chat_id == chat_id
                })
                .as_ref()
                .map(|unverified| unverified.member.user.debug_id())
                .unwrap_or_else(|| "{{user_not_in_unverified_users_map}}".to_owned());

            info_span!(
                "handle_callback_query_payload",
                %expected_user,
                allowed = payload.allowed,
            )
        };

        async {
            if payload.expected_user_id != callback_query.from.id {
                info!("User tried to reply to a capcha not meant for them",);
                return Ok(());
            }

            info!("User replied to captcha");

            UnverifiedUser::delete(&ctx, chat_id, user_id, DeleteReason::UserReplied).await?;

            if payload.allowed {
                info!("User passed captcha");
                return Ok(());
            }

            kick_user_due_to_captcha(bot, chat_id, user_id)
                .instrument(info_span!(
                    "kick_reason",
                    kick_reason = "captcha_wrong_answer"
                ))
                .await?;

            Ok::<_, Error>(())
        }
        .instrument(payload_span)
        .await
    }
    .await
    .map_err(Into::into)
}

#[instrument(skip_all, fields(chat = %msg.chat.debug_id()))]
pub(crate) async fn handle_new_chat_members(
    ctx: Arc<tg::Ctx>,
    msg: Message,
    users: Vec<User>,
) -> DynResult {
    async move {
        let image_url: &Url = &GREETING_ANIMATION_URL.parse().unwrap();

        let bot_id = ctx.bot.get_me().await?.id;

        let users: Vec<_> = users.into_iter().filter(|user| user.id != bot_id).collect();

        if let Some(user) = users.first() {
            let chat = ctx.bot.get_chat(msg.chat.id).await?;

            let is_captcha_enabled = ctx
                .tg_chats
                .get_or_update_captcha(db::TgChatQuery {
                    chat: &chat,
                    requested_by: user,
                    action: db::TgChatAction::HandleNewChatMember,
                })
                .await?;

            if !is_captcha_enabled {
                info!(
                    new_members = %users.iter().map(|user| user.debug_id()).join(", "),
                    "Captcha is disabled for this chat, ignoring new members"
                );
                return Ok(());
            }
        }

        let users = users.into_iter();

        let futs = users.map(|user| {
            let span = tracing::info_span!("user", user = %user.debug_id());
            let ctx = ctx.clone();
            async move {
                let mention = user.md_link();
                let chat_id = msg.chat.id;
                let user_id = user.id;
                let tg::Ctx { bot, captcha, .. } = &*ctx;

                let caption = [
                    &mention,
                    &markdown::escape(
                        "\nHi, new friend!\n\n\
                        Ответь на капчу: ",
                    ),
                    "*Кто должен победить в войне?*",
                ]
                .join("");

                let payload_allow = CaptchaReplyPayload {
                    expected_user_id: user_id,
                    allowed: true,
                };

                let payload_deny = CaptchaReplyPayload {
                    expected_user_id: user_id,
                    allowed: false,
                };

                let payload_allow = encoding::secure_encode(&payload_allow);
                let payload_deny = encoding::secure_encode(&payload_deny);

                let buttons = [[
                    InlineKeyboardButton::callback("Украина", payload_allow),
                    InlineKeyboardButton::callback("Россия", payload_deny),
                ]];

                let restricted_until_date = {
                    let restrictions_timeout =
                        CAPTCHA_VERIFICATION_TIMEOUT + CAPTCHA_RESTRICTIONS_TIMEOUT;

                    let restrictions_timeout =
                        chrono::Duration::from_std(restrictions_timeout).unwrap();

                    // Telegram works at seconds resolution, so we need to round up.
                    (Utc::now() + restrictions_timeout).round_subsecs(0)
                };

                let member = async {
                    let member = bot.get_chat_member(chat_id, user_id).await?;

                    bot.restrict_chat_member(chat_id, user.id, ChatPermissions::empty())
                        .until_date(restricted_until_date)
                        .await?;

                    Ok::<_, Error>(member)
                };

                let captcha_msg = bot
                    .send_animation(chat_id, InputFile::url(image_url.clone()))
                    .caption(caption)
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .reply_markup(ReplyMarkup::inline_kb(buttons))
                    .into_future()
                    .err_into();

                let (captcha_msg, member) = futures::try_join!(captcha_msg, member)?;

                let (send, recv) = oneshot::channel::<()>();

                let ctx = ctx.clone();
                let fut = async move {
                    if let Ok(recv_result) =
                        tokio::time::timeout(CAPTCHA_VERIFICATION_TIMEOUT, recv).await
                    {
                        if let Err(err) = recv_result {
                            warn!("BUG: captcha confirmation timeout channel closed: {err:#?}");
                        } else {
                            trace!("Captcha confirmation timeout successfully cancelled");
                        }
                        return;
                    }

                    info!(
                        captcha_timeout = format_args!("{CAPTCHA_VERIFICATION_TIMEOUT:.2?}"),
                        "Timed out waiting for captcha confirmation"
                    );

                    let delete_msg_result =
                        UnverifiedUser::delete(&ctx, chat_id, user_id, DeleteReason::Timeout).await;

                    let kick_result = kick_user_due_to_captcha(&ctx.bot, chat_id, user_id)
                        .instrument(info_span!("kick_reason", kick_reason = "captcha_timeout"))
                        .await;

                    if let Err(err) = delete_msg_result {
                        error!("Failed to remove captcha message: {err:#?}");
                    }

                    if let Err(err) = kick_result {
                        error!("Failed to ban user due to captcha: {err:#?}");
                    }
                }
                .in_current_span();

                tokio::spawn(fut);

                info!("Added user to captcha confirmation map");

                let unverified = UnverifiedUser {
                    member,
                    chat_id,
                    captcha_msg_id: captcha_msg.id,
                    captcha_timeout_cancel: Some(send),
                    restricted_until_date,
                };

                if let Some(old_unverified) = captcha
                    .unverified_users
                    .lock()
                    .insert((chat_id, user_id), unverified)
                {
                    warn!(
                        old_unverified = format_args!("{old_unverified:#?}"),
                        "BUG: user was already in captcha confirmation map"
                    );
                }

                Ok::<(), Error>(())
            }
            .instrument(span)
        });

        future::join_all(futs)
            .await
            .into_iter()
            .collect::<Result<Vec<()>>>()?;

        Ok::<_, Error>(())
    }
    .err_into()
    .await
}

#[instrument(skip_all)]
async fn kick_user_due_to_captcha(bot: &Bot, chat_id: ChatId, user_id: UserId) -> Result {
    let ban_timeout = Utc::now() + chrono::Duration::from_std(CAPTCHA_BAN_DURATION).unwrap();

    info!(until = %ban_timeout.to_human_readable(), "Kicking the user due to captcha...");

    bot.kick_chat_member(chat_id, user_id)
        .until_date(ban_timeout)
        .await?;

    Ok(())
}

#[instrument(skip_all, fields(
    chat = %msg.chat.debug_id(),
    user = %user.debug_id(),
))]
pub(crate) async fn handle_left_chat_member(
    ctx: Arc<tg::Ctx>,
    msg: Message,
    user: User,
) -> DynResult {
    async {
        let user_id = user.id;
        let chat_id = msg.chat.id;

        let bot_id = ctx.bot.get_me().await?.id;

        if user_id == bot_id {
            info!("Bot left the chat");
            ctx.captcha.clear_unverified_in_chat(chat_id);
            return Ok(());
        }

        info!(
            "Chat member left, canceling captcha confirmation \
            if one is still pending for them"
        );

        UnverifiedUser::delete(&ctx, chat_id, user_id, DeleteReason::UserLeftChat).await
    }
    .err_into()
    .await
}

enum DeleteReason {
    Timeout,
    UserLeftChat,
    UserReplied,
}

impl UnverifiedUser {
    async fn delete(
        ctx: &tg::Ctx,
        chat_id: ChatId,
        user_id: UserId,
        delete_reason: DeleteReason,
    ) -> Result {
        let Some(mut unverified) = Self::delete_from_map(ctx, chat_id, user_id) else {
            if let DeleteReason::UserReplied = delete_reason {
                warn!("User replied to captcha, but they weren't in the unverified users map");
            }
            return Ok(());
        };

        let msg_id = unverified.captcha_msg_id;

        if let DeleteReason::UserLeftChat | DeleteReason::UserReplied = delete_reason {
            let sender = unverified.captcha_timeout_cancel.take();
            if let Some(Err(())) = sender.map(|sender| sender.send(())) {
                debug!("Captcha time out task already finished (receiver dropped)");
            }
        }

        info!(
            msg_id = msg_id.to_tracing(),
            "Deleting captcha message and restoring original user permissions..."
        );

        let (a, b) = futures::join!(
            unverified.restore_original_perms(&ctx.bot),
            ctx.bot
                .delete_message(unverified.chat_id, msg_id)
                .into_future()
                .err_into(),
        );

        let errs: Vec<_> = a.err().into_iter().chain(b.err()).collect();

        match <[_; 1]>::try_from(errs) {
            Err(errs) if errs.is_empty() => Ok(()),
            Ok([err]) => Err(err),
            Err(errs) => Err(err!(ErrorKind::Multiple { errs })),
        }
    }

    #[instrument(skip_all)]
    fn delete_from_map(ctx: &tg::Ctx, chat_id: ChatId, user_id: UserId) -> Option<Self> {
        let unverified = ctx
            .captcha
            .unverified_users
            .lock()
            .remove(&(chat_id, user_id));

        if unverified.is_none() {
            debug!("User was not in unverified users map, thus no captcha to resolve");
        }

        unverified
    }

    #[instrument(skip_all, fields(unverified = format_args!("{self:#?}")))]
    async fn restore_original_perms(&self, bot: &Bot) -> Result {
        let chat_id = self.chat_id;
        let user_id = self.member.user.id;

        let current = bot.get_chat_member(chat_id, user_id).await?;

        self.restore_original_perms_with_current(bot, &current)
            .await
    }

    #[instrument(skip_all, fields(current_user = format_args!("{current:#?}")))]
    async fn restore_original_perms_with_current(&self, bot: &Bot, current: &ChatMember) -> Result {
        let user_modified = !matches!(
            &current.kind,
            ChatMemberKind::Restricted(restricted)
            if restricted.until_date == UntilDate::Date(self.restricted_until_date)
                && restricted_to_chat_perms(restricted) == ChatPermissions::empty()
        );

        if user_modified {
            info!("Unverified user was modified, so not restoring original permissions");
            return Ok(());
        };

        let chat_id = self.chat_id;
        let user_id = self.member.user.id;

        match &self.member.kind {
            ChatMemberKind::Member(_) => {
                info!("Restoring original member permissions");

                bot.restrict_chat_member(chat_id, user_id, ChatPermissions::all())
                    .await?;
            }
            ChatMemberKind::Restricted(original_restricted) => {
                info!("Restoring original restricted permissions");

                let perms = restricted_to_chat_perms(original_restricted);

                let until_date = match original_restricted.until_date {
                    UntilDate::Date(date) => date,
                    UntilDate::Forever => chrono::Utc.timestamp_opt(0, 0).unwrap(),
                };

                bot.restrict_chat_member(chat_id, user_id, perms)
                    .until_date(until_date)
                    .await?;
            }
            ChatMemberKind::Owner(_)
            | ChatMemberKind::Administrator(_)
            | ChatMemberKind::Left
            | ChatMemberKind::Banned(_) => {
                warn!("Unexpected chat member kind for captcha");
                return Ok(());
            }
        }

        Ok(())
    }
}

fn restricted_to_chat_perms(restricted: &Restricted) -> ChatPermissions {
    let Restricted {
        until_date: _,
        is_member: _,
        can_send_messages,
        can_send_audios,
        can_send_documents,
        can_send_photos,
        can_send_videos,
        can_send_video_notes,
        can_send_voice_notes,
        can_send_other_messages,
        can_add_web_page_previews,
        can_change_info,
        can_invite_users,
        can_pin_messages,
        can_manage_topics,
        can_send_polls,
    } = restricted;

    #[rustfmt::skip]
    let perms = [
        (can_send_messages,         ChatPermissions::SEND_MESSAGES),
        (can_send_audios,           ChatPermissions::SEND_AUDIOS),
        (can_send_documents,        ChatPermissions::SEND_DOCUMENTS),
        (can_send_photos,           ChatPermissions::SEND_PHOTOS),
        (can_send_videos,           ChatPermissions::SEND_VIDEOS),
        (can_send_video_notes,      ChatPermissions::SEND_VIDEO_NOTES),
        (can_send_voice_notes,      ChatPermissions::SEND_VOICE_NOTES),
        (can_send_other_messages,   ChatPermissions::SEND_OTHER_MESSAGES),
        (can_add_web_page_previews, ChatPermissions::ADD_WEB_PAGE_PREVIEWS),
        (can_change_info,           ChatPermissions::CHANGE_INFO),
        (can_invite_users,          ChatPermissions::INVITE_USERS),
        (can_pin_messages,          ChatPermissions::PIN_MESSAGES),
        (can_manage_topics,         ChatPermissions::MANAGE_TOPICS),
        (can_send_polls,            ChatPermissions::SEND_POLLS),
    ];

    perms
        .into_iter()
        .filter(|(&enabled, _)| enabled)
        .map(|(_, perm)| perm)
        .collect()
}

/// The following methods are used only for maintenance purposes as an escape
/// hatch to try remediate the system in case of a bug.
impl CaptchaCtx {
    pub(crate) fn list_unverified(&self) -> Vec<(ChatId, User)> {
        self.unverified_users
            .lock()
            .values()
            .map(|unverified| (unverified.chat_id, unverified.member.user.clone()))
            .collect()
    }

    pub(crate) fn clear_unverified_in_chat(&self, target_chat: ChatId) {
        let mut unverified_users = self.unverified_users.lock();
        let prev_len = unverified_users.len();
        unverified_users.retain(|(chat_id, _), _| *chat_id != target_chat);

        info!(
            total_cleared = prev_len - unverified_users.len(),
            "Cleared all unverified users in chat"
        );
    }

    #[instrument(skip_all)]
    pub(crate) fn clear_unverified(&self) {
        info!("Clearing all unverified users");
        let unverified_users = std::mem::take(&mut *self.unverified_users.lock());
        for mut unverified in unverified_users.into_values() {
            let Some(cancel) = unverified.captcha_timeout_cancel.take() else {
                continue;
            };
            if let Err(()) = cancel.send(()) {
                let user = unverified.member.user.debug_id();
                let chat_id = unverified.chat_id;
                let msg_id = unverified.captcha_msg_id;
                warn!(%user, %chat_id, %msg_id, "Failed to cancel captcha timeout");
            }
        }
    }
}
