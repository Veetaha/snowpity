use crate::tg::Bot;
use crate::util;
use crate::util::prelude::*;
use crate::util::DynError;
use crate::Error;
use crate::Result;
use chrono::prelude::*;
use futures::prelude::*;
use once_cell::sync::Lazy as SyncLazy;
use parking_lot::Mutex as SyncMutex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::{
    ChatPermissions, InlineKeyboardButton, InputFile, Message, ReplyMarkup, User,
};
use teloxide::utils::markdown;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::info_span;
use tracing::instrument;
use tracing::trace;
use tracing::warn;
use tracing_futures::Instrument;

/// Duration for the new users to solve the captcha. If they don't reply
/// in this time, they will be kicked.
const CAPTCHA_TIMEOUT: Duration = Duration::from_secs(60);
const CAPTCHA_DURATION_TEXT: &str = "1 минута";

/// Duration for the ban of the users that didn't pass captcha.
const CAPTCHA_BAN_DURATION: Duration = Duration::from_secs(60 * 2);

const GREETING_ANIMATION_URL: &str = "https://derpicdn.net/img/2021/12/19/2767482/small.gif";

static UNVERIFIED_USERS: SyncLazy<
    SyncMutex<HashMap<(ChatId, UserId), (i32, oneshot::Sender<()>)>>,
> = SyncLazy::new(Default::default);

#[derive(Serialize, Deserialize, Debug)]
struct CaptchaReplyPayload {
    expected_user_id: UserId,
    allowed: bool,
}

#[instrument(
    skip(bot, callback_query),
    fields(
        from = callback_query.from.debug_id().as_str(),
        msg = callback_query.message.as_ref().and_then(|msg| msg.text()),
    )
)]
pub(crate) async fn handle_callback_query(
    bot: Bot,
    callback_query: CallbackQuery,
) -> Result<(), Box<DynError>> {
    async {
        debug!("Processing callback query");

        let callback_data = match callback_query.data {
            Some(data) => data,
            None => {
                warn!("Received empty callback data");
                return Ok(());
            }
        };

        let captcha_message = match callback_query.message {
            Some(message) => message,
            None => {
                warn!("Received empty callback message");
                return Ok(());
            }
        };

        let payload: CaptchaReplyPayload = util::encoding::secure_decode(&callback_data)?;

        let payload_span = info_span!("handle_callback_query_payload", ?payload);

        async {
            if payload.expected_user_id != callback_query.from.id {
                info!(
                    user_id = %callback_query.from.id,
                    "User tried to reply to a capcha not meant for them",
                );
                return Ok(());
            }

            let user_id = callback_query.from.id;
            let chat_id = captcha_message.chat.id;

            cancel_captcha_confirmation(&bot, chat_id, user_id).await?;

            if !payload.allowed {
                info!("User chose wrong answer in captcha, kicking them...");
                kick_user_due_to_captcha(&bot, chat_id, user_id).await?;
                return Ok(());
            }

            info!("User passed captcha");

            let default_perms = match bot.get_chat(chat_id).await?.permissions() {
                Some(perms) => perms,
                None => {
                    warn!("Could not get default chat member permissions",);
                    return Ok(());
                }
            };

            bot.restrict_chat_member(chat_id, user_id, default_perms)
                .await?;

            Ok::<_, Error>(())
        }
        .instrument(payload_span)
        .await
    }
    .await
    .map_err(Into::into)
}

pub(crate) async fn handle_new_chat_members(
    bot: Bot,
    msg: Message,
    users: Vec<User>,
) -> Result<(), Box<DynError>> {
    async {
        let image_url: Url = GREETING_ANIMATION_URL.parse().unwrapx();

        let futs = users.iter().map(|user| async {
            let mention = user.md_link();
            let chat_id = msg.chat.id;
            let user_id = user.id;
            let chat_username = msg.chat.username();

            let span = tracing::info_span!("handle_new_chat_members", %chat_id, chat_username, %mention);

            async {
                let caption = format!(
                    "{}{}{}{}",
                    mention,
                    markdown::escape(
                        "\nHi, new friend! Привет, поняша :3\n\n\
                        Ответь на капчу: "
                    ),
                    "*Путин это кто?*",
                    markdown::escape(&format!(
                        "\n\nУ тебя {CAPTCHA_DURATION_TEXT} на правильный ответ, иначе будешь кикнут.",
                    ))
                );

                let payload_allow = CaptchaReplyPayload {
                    expected_user_id: user_id,
                    allowed: true,
                };

                let payload_deny = CaptchaReplyPayload {
                    expected_user_id: user_id,
                    allowed: false,
                };

                let payload_allow = util::encoding::secure_encode(&payload_allow);
                let payload_deny = util::encoding::secure_encode(&payload_deny);

                let buttons = [[
                    InlineKeyboardButton::callback("Хуйло! 😉", payload_allow),
                    InlineKeyboardButton::callback("Молодец (бан)! 🤨", payload_deny),
                ]];

                bot.restrict_chat_member(chat_id, user.id, ChatPermissions::empty())
                    .await?;

                let captcha_message_id = bot
                    .send_animation(chat_id, InputFile::url(image_url.clone()))
                    .caption(caption)
                    .reply_to_message_id(msg.id)
                    .reply_markup(ReplyMarkup::inline_kb(buttons))
                    .await?
                    .id;

                let (send, recv) = oneshot::channel::<()>();

                let bot = bot.clone();

                let fut = async move {
                    if let Ok(recv_result) = tokio::time::timeout(CAPTCHA_TIMEOUT, recv).await {
                        if let Err(err) = recv_result {
                            warn!("BUG: captcha confirmation timeout channel closed: {err:#?}");
                        } else {
                            trace!("Captcha confirmation timeout succefully cancelled");
                        }
                        return;
                    }

                    debug!(
                        captcha_timeout = format_args!("{CAPTCHA_TIMEOUT:.2?}"),
                        "Timed out waiting for captcha confirmation"
                    );

                    let (delete_message_result, kick_result) = futures::join!(
                        cancel_captcha_confirmation(&bot, chat_id, user_id),
                        kick_user_due_to_captcha(&bot, chat_id, user_id)
                    );

                    if let Err(err) = delete_message_result {
                        error!("Failed to remove captcha message: {err:#?}");
                    }

                    if let Err(err) = kick_result {
                        error!("Failed to ban user due to captcha: {err:#?}");
                    }
                }
                .in_current_span();

                tokio::spawn(fut);

                info!("Added user to captcha confirmation map");

                UNVERIFIED_USERS
                    .lock()
                    .insert((chat_id, user_id), (captcha_message_id, send));

                Ok::<_, Error>(())
            }
            .instrument(span)
            .await
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

#[instrument(skip(bot))]
async fn kick_user_due_to_captcha(bot: &Bot, chat_id: ChatId, user_id: UserId) -> Result {
    let ban_timeout = Utc::now() + chrono::Duration::from_std(CAPTCHA_BAN_DURATION).unwrapx();

    info!(until = ban_timeout.to_ymd_hms().as_str(), "Banning user");

    bot.kick_chat_member(chat_id, user_id)
        .until_date(ban_timeout)
        .await?;

    Ok(())
}

pub(crate) async fn handle_left_chat_member(
    bot: Bot,
    msg: Message,
    user: User,
) -> Result<(), Box<DynError>> {
    async {
        let user_id = user.id;
        let chat_id = msg.chat.id;

        info!("Chat member left, canceling captcha confirmation if they didn't pass it");

        cancel_captcha_confirmation(&bot, chat_id, user_id).await?;

        Ok::<_, Error>(())
    }
    .err_into()
    .await
}

#[instrument(skip(bot))]
async fn cancel_captcha_confirmation(bot: &Bot, chat_id: ChatId, user_id: UserId) -> Result {
    let result = UNVERIFIED_USERS.lock().remove(&(chat_id, user_id));

    let (msg_id, send) = match result {
        Some(res) => res,
        None => {
            debug!("User was not in unverified users map, thus no captcha to cancel");
            return Ok(());
        }
    };

    if let Err(()) = send.send(()) {
        debug!("Failed to cancel captcha time out (receiver dropped)");
    }

    info!(
        %msg_id,
        "Cancelled captcha confirmation, deleting captcha message"
    );

    bot.delete_message(chat_id, msg_id).await?;

    Ok(())
}
