mod banned_phrases;

use crate::tg;
use crate::util::prelude::*;
use crate::Result;
use async_trait::async_trait;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "snake_case",
    description = "Следующие команды доступны для администраторов:"
)]
pub(crate) enum Cmd {
    #[command(description = "показать этот текст")]
    AdminHelp,

    #[command(description = "добавить фразу в список запрещенных")]
    AddBannedPhrase(String),

    #[command(description = "показать список запрещённых фраз")]
    ListBannedPhrases,

    #[command(description = "удалить фразу из списка запрещённых")]
    DeleteBannedPhrase(String),

    #[command(description = "добавить фразу в список исключений")]
    AddExceptionalPhrase(String),

    #[command(description = "показать список исключений")]
    ListExceptionalPhrases,

    #[command(description = "удалить фразу из списка исключений")]
    DeleteExceptionalPhrase(String),
}

#[async_trait]
impl tg::cmd::Command for Cmd {
    async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> Result {
        match self {
            Cmd::AdminHelp => {
                ctx.bot.reply_help_md_escaped::<Cmd>(msg).await?;
            }
            Cmd::AddBannedPhrase(phrase) => {
                banned_phrases::add_banned_phrase(ctx, msg, phrase).await?;
            }
            Cmd::ListBannedPhrases => {
                banned_phrases::list_banned_phrases(ctx, msg).await?;
            }
            Cmd::DeleteBannedPhrase(phrase) => {
                banned_phrases::delete_banned_phrase(ctx, msg, phrase).await?;
            }
            Cmd::AddExceptionalPhrase(phrase) => {
                banned_phrases::add_exceptional_phrase(ctx, msg, phrase).await?;
            }
            Cmd::ListExceptionalPhrases => {
                banned_phrases::list_exceptional_phrases(ctx, msg).await?;
            }
            Cmd::DeleteExceptionalPhrase(phrase) => {
                banned_phrases::delete_exceptional_phrase(ctx, msg, phrase).await?;
            }
        }
        Ok(())
    }
}
