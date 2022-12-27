mod ftai;

use crate::tg;
use crate::Result;
use async_trait::async_trait;
use ftai::FtaiCmd;
use teloxide::prelude::*;
use teloxide::types::InputFile;
use teloxide::utils::command::BotCommands;
use teloxide::utils::markdown;

const HELP_ANIMATION_URL: &str = "https://user-images.githubusercontent.com/36276403/209577979-b0ace368-4bea-4a10-a687-d3f24cbed6a2.mp4";

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "snake_case",
    description = "The following commands are available:"
)]
pub(crate) enum Cmd {
    #[command(description = "display this text")]
    Help,
    // #[command(description = "Generate audio via 15.ai: <character name>,<text>")]
    // Ftai(String),
}

// 15 AI is very unstable, and not available now. We should come up with a way
// to automatically keep track of its availability, and disable/enable the
// 15.ai command accordingly to avoid unnecessarily displaying errors to users
#[allow(dead_code)]
fn ignore_15_ai(cmd: &str, ctx: &tg::Ctx, msg: &Message) {
    let _ = cmd.parse::<FtaiCmd>().unwrap().handle(ctx, msg);
}

#[async_trait]
impl tg::cmd::Command for Cmd {
    async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> Result {
        match self {
            Cmd::Help => {
                let bot_username = ctx
                    .bot
                    .get_me()
                    .await?
                    .user
                    .username
                    .expect("BUG: bot is guaranteed have a username");

                let help_text = markdown::escape(
                    &format!("\
                        {}\n\n\
                        You can also write a message like this to share an image or GIF from derpibooru \
                        (copy the following line to test): \n\
                        \n\
                        @{bot_username} https://derpibooru.org/1975357",
                    Cmd::descriptions())
                );

                let animation = InputFile::url(HELP_ANIMATION_URL.parse().unwrap());

                ctx.bot
                    .send_animation(msg.chat.id, animation)
                    .reply_to_message_id(msg.id)
                    .caption(help_text)
                    .await?;
            } // Cmd::Ftai(cmd) => {

              //     cmd.parse::<FtaiCmd>()?.handle(ctx, msg).await?

              // }
        }
        Ok(())
    }
}
