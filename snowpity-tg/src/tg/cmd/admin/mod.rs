use crate::prelude::*;
use crate::{db, tg, Error, Result};
use async_trait::async_trait;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ChatMemberKind;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "snake_case",
    description = "The following commands are available to the chat owner:"
)]
pub(crate) enum Cmd {
    #[command(description = "display this text")]
    AdminHelp,

    #[command(description = "\
        enables (if disabled) or disables (if enabled) \
        the captcha verification for new users")]
    ToggleCaptcha,

    #[command(description = "shows the configurations of the current chat")]
    ChatConfig,
}

pub(crate) async fn filter(ctx: Arc<tg::Ctx>, msg: Message) -> bool {
    async {
        let user_kind = ctx
            .bot
            .get_chat_member(msg.chat.id, msg.from().unwrap().id)
            .await?
            .kind;

        // As for now, we allow admin actions only for chat owners
        Ok::<_, Error>(matches!(user_kind, ChatMemberKind::Owner { .. }))
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
            Cmd::AdminHelp => {
                ctx.bot.reply_help_md_escaped::<Cmd>(msg).await?;
            }
            Cmd::ToggleCaptcha => {
                let tg_chat_ctx = tg_chat_ctx(db::TgChatAction::ToggleCaptchaCommand);

                ctx.db.tg_chat.get_or_update_captcha(tg_chat_ctx).await?;

                ctx.captcha.clear_unverified_in_chat(msg.chat.id);
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
    crate::util::encoding::to_yaml_string(&chat)
}
