mod banned_words;
mod ftai;

use crate::tg;
use crate::util::prelude::*;
use crate::Result;
use async_trait::async_trait;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::utils::markdown;

use self::ftai::FtaiCmd;

#[derive(BotCommands, Clone, Debug)]
#[command(rename = "snake_case", description = "Следующие команды доступны:")]
pub(crate) enum Cmd {
    #[command(description = "показать этот текст")]
    Help,

    #[command(description = "Сгенерировать аудио с помощью 15.ai: <персонаж>,<текст>")]
    Ftai(String),

    #[command(description = "запретить сообщения, которые включают в себя заданное слово")]
    BanWord(String),

    #[command(description = "показать список всех запрещённых слов")]
    BannedWords,

    #[command(description = "удалить слово из списка запрещённых")]
    UnbanWord(String),

    // #[command(description = "\
    //     запретить сообщения, которые подходят под образец (используется \
    //     [синтаксис Rust regex](\"https://docs.rs/regex/latest/regex/#syntax))")]
    // BanRegex(String),

    // #[command(description = "показать список всех запрещённых образцов (regex)")]
    // BannedRegexes,

    // #[command(description = "удалить образец (regex) из списка запрещённых")]
    // UnbanRegex(String),
}

#[async_trait]
impl tg::cmd::Command for Cmd {
    async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> Result {
        match self {
            Cmd::Help => {
                ctx.bot
                    .reply_chunked(&msg, markdown::escape(&Cmd::descriptions().to_string()))
                    .disable_web_page_preview(false)
                    // .parse_mode(ParseMode::Html)
                    .await?;
            }
            Cmd::Ftai(cmd) => cmd.parse::<FtaiCmd>()?.handle(ctx, msg).await?,
            Cmd::BanWord(word) => {
                banned_words::ban_word(ctx, msg, word).await?;
            }
            Cmd::BannedWords => {
                banned_words::banned_words(ctx, msg).await?;
            }
            Cmd::UnbanWord(word) => {
                banned_words::unban_word(ctx, msg, word).await?;
            }
        }
        Ok(())
    }
}
