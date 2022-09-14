use crate::db::db_constraints;
use crate::util::prelude::*;
use crate::{banned_words, err_val, Result, UserError};
use chrono::prelude::*;
use futures::prelude::*;
use teloxide::types::{ChatId, UserId};
use tracing::instrument;

#[derive(Debug)]
pub(crate) struct BannedWord {
    pub(crate) word: String,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) created_by: UserId,
}

pub(crate) struct TgChatBannedWordsRepo {
    pool: sqlx::PgPool,
}

impl TgChatBannedWordsRepo {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self))]
    pub(crate) async fn create(
        &self,
        chat_id: ChatId,
        word: &banned_words::Word,
        created_by: UserId,
    ) -> Result {
        let query = sqlx::query!(
            "INSERT INTO tg_chat_banned_words (tg_chat_id, word, created_by)
            VALUES ($1, $2, $3)",
            chat_id.into_db(),
            word.as_str(),
            created_by.to_string(),
        );

        query.execute(&self.pool).await.map_err(|err| {
            if err.is_constraint_violation(db_constraints::TG_CHAT_AND_BANNED_WORD_COMPOSITE_PK) {
                return err_val!(UserError::BannedWordAlreadyExists { word });
            }
            err.into()
        })?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub(crate) fn get_all_by_chat_id(
        &self,
        chat_id: ChatId,
    ) -> impl Stream<Item = Result<BannedWord>> + '_ {
        let query = sqlx::query!(
            "SELECT word, created_at, created_by
            FROM tg_chat_banned_words WHERE tg_chat_id = $1",
            chat_id.into_db(),
        );

        query
            .fetch(&self.pool)
            .map_ok(|record| BannedWord {
                word: record.word,
                created_at: record.created_at,
                created_by: record.created_by.into_app_or_panic(),
            })
            .err_into()
    }

    #[instrument(skip(self))]
    pub(crate) async fn delete(&self, chat_id: ChatId, word: banned_words::Word) -> Result {
        let query = sqlx::query!(
            "DELETE FROM tg_chat_banned_words
            WHERE tg_chat_id = $1 AND word = $2",
            chat_id.to_string(),
            word.clone().into_db(),
        );

        if query.execute(&self.pool).await?.rows_affected() == 0 {
            return Err(err_val!(UserError::BannedWordNotFound {
                word
            }));
        }

        Ok(())
    }
}
