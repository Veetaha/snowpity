use crate::prelude::*;
use crate::{err, tg, Error, Result};
use std::str::FromStr;
use teloxide::prelude::*;
use teloxide::types::{InputFile, Message};
use teloxide::utils::markdown;

#[derive(Debug, Clone)]
pub(crate) struct FtaiCmd {
    character: String,
    text: String,
}

impl FromStr for FtaiCmd {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        let (character, text) = input
            .split_once(',')
            .ok_or_else(|| err!(FtaiCommandError::FtaiInvalidFormat))?;

        let character = character.trim();
        let text = text.trim();

        if text.len() > crate::ftai::MAX_TEXT_LENGTH {
            return Err(err!(FtaiCommandError::FtaiTextTooLong {
                actual_len: text.len()
            }));
        }

        let text = text.to_owned();

        if !text.contains('{') && !text.contains('}') && text.chars().any(char::is_numeric) {
            return Err(err!(FtaiCommandError::FtaiTextContainsNumber));
        }

        Ok(FtaiCmd {
            character: character.to_owned(),
            text,
        })
    }
}

impl FtaiCmd {
    #[instrument(skip(ctx, msg))]
    pub(crate) async fn handle(self, ctx: &tg::Ctx, msg: &Message) -> Result {
        info!("Generating audio via 15.ai");

        let start = std::time::Instant::now();

        let ogg = ctx.ftai.generate_audio(&self.character, &self.text).await?;

        let text_len = self.text.len();

        let took = format!("{:.2?}", start.elapsed());

        let caption = markdown::escape(&format!("symbols: {text_len}, took: {took}"));

        let input_file = InputFile::memory(ogg.data).file_name("voice.ogg");

        ctx.bot
            .send_voice(msg.chat.id, input_file)
            .caption(caption)
            .reply_to_message_id(msg.id)
            .await?;

        info!(%took, "Finished generating audio via 15.ai");

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FtaiCommandError {
    #[error("The text for 15.ai must not contain digits except for ARPAbet notation")]
    FtaiTextContainsNumber,

    #[error(
        "The text for 15.ai must have less than {} symbols. The length of your text is {actual_len}",
        crate::ftai::MAX_TEXT_LENGTH
    )]
    FtaiTextTooLong { actual_len: usize },

    #[error("The command for 15.ai must have the character name, a comma (,) and the text: <character name>,<text>")]
    FtaiInvalidFormat,
}
