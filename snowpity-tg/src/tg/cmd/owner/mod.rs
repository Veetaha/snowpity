use crate::prelude::*;
use crate::{db, encoding, tg, Error, Result};
use async_trait::async_trait;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ChatMemberKind;
use teloxide::utils::command::BotCommands;
use teloxide::utils::markdown;
use tracing::field;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "snake_case",
    description = "The following commands are available to the chat owner:"
)]
pub(crate) enum Cmd {
    #[command(description = "display this text")]
    OwnerHelp,

    #[command(description = "\
        enables (if disabled) or disables (if enabled) \
        the captcha verification for new users")]
    ToggleCaptcha,

    #[command(description = "shows the configurations of the current chat")]
    ChatConfig,
}

#[instrument(skip_all, fields(chat = %msg.chat.debug_id(), user, user_kind))]
pub(crate) async fn filter(ctx: Arc<tg::Ctx>, msg: Message) -> bool {
    async {
        let user = ctx
            .bot
            .get_chat_member(msg.chat.id, msg.from().unwrap().id)
            .await?;

        let span = tracing::Span::current();

        span.record("user_kind", field::debug(user.kind.status()));
        span.record("user", field::display(user.user.debug_id()));

        let is_owner = matches!(user.kind, ChatMemberKind::Owner { .. });

        if !is_owner {
            info!("Non-owner user tried to access owner command");
        }

        Ok::<_, Error>(is_owner)
    }
    .await
    .unwrap_or_else(|err| {
        error!(
            err = tracing_err(&err),
            "Couldn't get chat member info, conservatively denying admin access"
        );
        false
    })
}

#[async_trait]
impl tg::cmd::Command for Cmd {
    async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> Result {
        let tg_chat_ctx = |action| db::TgChatQuery {
            chat: &msg.chat,
            requested_by: msg.from().unwrap(),
            action,
        };

        match self {
            Cmd::OwnerHelp => {
                ctx.bot.reply_help_md_escaped::<Cmd>(msg).await?;
            }
            Cmd::ToggleCaptcha => {
                // FIXME: verify the bot has enough permissions to do captcha
                // verification in the chat

                let tg_chat_ctx = tg_chat_ctx(db::TgChatAction::ToggleCaptchaCommand);

                let is_captcha_enabled = ctx.db.tg_chat.get_or_update_captcha(tg_chat_ctx).await?;
                let enabled = if is_captcha_enabled {
                    "enabled"
                } else {
                    "disabled"
                };

                let text = format!("Captcha verification is now `{enabled}`");

                ctx.bot.send_message(msg.chat.id, text).await?;
            }
            Cmd::ChatConfig => {
                let tg_chat_ctx = tg_chat_ctx(db::TgChatAction::ChatConfigCommand);

                let chat = ctx.db.tg_chat.get_chat(tg_chat_ctx).await?;

                ctx.bot
                    .send_message(msg.chat.id, display_chat(chat))
                    .await?;
            }
        }
        Ok(())
    }
}

fn display_chat(chat: db::TgChat) -> String {
    markdown::code_block_with_lang(&encoding::to_yaml_string(&chat), "yml")
}
