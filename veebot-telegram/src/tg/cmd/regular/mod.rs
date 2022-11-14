mod ftai;

use crate::tg;
use crate::util::prelude::*;
use crate::Result;
use async_trait::async_trait;
use ftai::FtaiCmd;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "snake_case",
    description = "Следующие команды доступны:"
)]
pub(crate) enum Cmd {
    #[command(description = "показать этот текст")]
    Help,

    #[command(description = "Сгенерировать аудио с помощью 15.ai: <персонаж>,<текст>")]
    Ftai(String),
}

#[async_trait]
impl tg::cmd::Command for Cmd {
    async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> Result {
        match self {
            Cmd::Help => {
                ctx.bot.reply_help_md_escaped::<Cmd>(msg).await?;
            }
            Cmd::Ftai(cmd) => cmd.parse::<FtaiCmd>()?.handle(ctx, msg).await?,
        }
        Ok(())
    }
}
