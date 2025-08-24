mod ftai;

use crate::tg;
use crate::Result;
use async_trait::async_trait;
use ftai::FtaiCmd;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InputFile, ReplyMarkup, ReplyParameters};
use teloxide::utils::command::BotCommands;
use teloxide::utils::markdown;

pub(crate) use ftai::FtaiCommandError;

const HELP_ANIMATION_URL: &str = "https://user-images.githubusercontent.com/36276403/209577979-b0ace368-4bea-4a10-a687-d3f24cbed6a2.mp4";
const EXAMPLE_DERPIBOORU_MEDIA_URL: &str = "https://derpibooru.org/1975357";
const EXAMPLE_TWITTER_MEDIA_URL: &str = "https://twitter.com/Sethisto/status/1558884492190035968";
const EXAMPLE_DEVIANT_ART_MEDIA_URL: &str =
    "https://www.deviantart.com/mandumustbasukanemen/art/hay-station-895499143";

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case", description = "Commands:")]
pub(crate) enum Cmd {
    #[command(description = "show the guide")]
    Help,
    // #[command(description = "Generate audio via 15.ai: <character name>,<text>")]
    // Ftai(String),
}

// 15 AI is very unstable, and not available now. We should come up with a way
// to automatically keep track of its availability, and disable/enable the
// 15.ai command accordingly to avoid unnecessarily displaying errors to users
#[allow(dead_code)]
fn ignore_15_ai(cmd: &str, ctx: &tg::Ctx, msg: &Message) {
    drop(cmd.parse::<FtaiCmd>().unwrap().handle(ctx, msg));
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

                let commands = Cmd::descriptions();

                let header = markdown::escape(&format!(
                    "{commands}\n\n\
                    Write this to share images/GIFs/videos by link:",
                ));

                let example_usage = markdown::code_inline(&format!("@{bot_username} {{link}}"));

                let help_text = format!("{header}\n\n{example_usage}");

                let animation = InputFile::url(HELP_ANIMATION_URL.parse().unwrap());

                let examples = [
                    (EXAMPLE_DERPIBOORU_MEDIA_URL, "Derpibooru"),
                    (EXAMPLE_TWITTER_MEDIA_URL, "Twitter"),
                    (EXAMPLE_DEVIANT_ART_MEDIA_URL, "DeviantArt"),
                ];

                let buttons = examples.map(|(url, platform)| {
                    let text = format!("See {platform} example");
                    [InlineKeyboardButton::switch_inline_query_current_chat(
                        text, url,
                    )]
                });

                ctx.bot
                    .send_animation(msg.chat.id, animation)
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .caption(help_text)
                    .reply_markup(ReplyMarkup::inline_kb(buttons))
                    .await?;
            } // Cmd::Ftai(cmd) => {

              //     cmd.parse::<FtaiCmd>()?.handle(ctx, msg).await?

              // }
        }
        Ok(())
    }
}
