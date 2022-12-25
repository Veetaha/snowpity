mod ftai;

use crate::prelude::*;
use crate::tg;
use crate::Result;
use async_trait::async_trait;
use ftai::FtaiCmd;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "snake_case",
    description = "The following commands are available:"
)]
pub(crate) enum Cmd {
    #[command(description = "display this text")]
    Help,

    #[command(description = "Generate audio via 15.ai: <character name>,<text>")]
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
