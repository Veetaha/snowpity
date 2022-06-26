use crate::{err_val, tg, Error, Result, UserError};
use std::str::FromStr;
use teloxide::prelude::*;
use teloxide::types::{InputFile, Message};
use teloxide::utils::markdown;
use tracing::{info, instrument};

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
            .ok_or_else(|| err_val!(UserError::FtaiInvalidFormat))?;

        let character = character.trim();
        let text = text.trim();

        if text.len() > crate::ftai::MAX_TEXT_LENGTH {
            return Err(err_val!(UserError::FtaiTextTooLong {
                actual_len: text.len()
            }));
        }

        let text = text.to_owned();

        if text.chars().any(char::is_numeric) {
            return Err(err_val!(UserError::FtaiTextContainsNumber));
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

        let caption = markdown::escape(&format!("символов: {text_len}, заняло: {took}"));

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
