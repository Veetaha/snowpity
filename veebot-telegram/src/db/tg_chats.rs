use crate::db::db_constraints;
use crate::util::prelude::*;
use crate::util::PgQuery;
use crate::Result;
use crate::{err_val, UserError};
use chrono::prelude::*;
use futures::prelude::*;
use sqlx::postgres::types::PgInterval;
use std::time::Duration;
use teloxide::types::{ChatId, UserId};
use tracing::{instrument, warn};

struct TgChat {
    pub(crate) id: ChatId,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) created_by: UserId,
    pub(crate) banned_pattern_mute_duration: Option<Duration>,
}

struct TgChatRecord {
    id: String,
    created_at: DateTime<Utc>,
    created_by: String,
    banned_pattern_mute_duration: Option<PgInterval>,
}

impl FromDb<TgChatRecord> for TgChat {
    fn from_db(record: TgChatRecord) -> Self {
        TgChat {
            id: record.id.into_app(),
            created_at: record.created_at,
            created_by: record.created_by.into_app(),
            banned_pattern_mute_duration: record.banned_pattern_mute_duration.into_app(),
        }
    }
}

pub(crate) struct TgChatsRepo {
    pool: sqlx::PgPool,
}

impl TgChatsRepo {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self))]
    pub(crate) async fn create(
        &self,
        chat_id: ChatId,
        created_by: UserId,
        banned_pattern_mute_duration: Option<Duration>,
    ) -> Result {
        let query = sqlx::query!(
            "INSERT INTO tg_chats (id, created_by, banned_pattern_mute_duration)
            VALUES ($1, $2, $3)",
            chat_id.into_db(),
            created_by.into_db(),
            banned_pattern_mute_duration.try_into_db()?,
        );

        query.execute(&self.pool).await.map_err(|err| {
            if err.is_constraint_violation(db_constraints::TG_CHATS_PK) {
                return err_val!(UserError::ChatAlreadyExists { chat_id });
            }
            err.into()
        })?;

        Ok(())
    }

    pub(crate) async fn get_by_id(&self, chat_id: ChatId) -> Result<TgChat> {
        let query = sqlx::query_as!(
            TgChatRecord,
            "SELECT id, created_at, created_by, banned_pattern_mute_duration
            FROM tg_chats
            WHERE id = $1",
            chat_id.into_db(),
        );

        query.fetch_one(&self.pool).await.map(IntoApp::into_app)
    }

    /// Get all chats from tg_chats table
    #[instrument(skip(self))]
    pub(crate) async fn get_all(&self) -> Result<Vec<TgChat>> {
        let query = sqlx::query_as!(
            TgChatRecord,
            "SELECT id, created_by, created_at, banned_pattern_mute_duration
            FROM tg_chats"
        );

        query
            .fetch(&self.pool)
            .map_ok(IntoApp::into_app)
            .try_collect()
            .await
            .map_err(Into::into)
    }

    /// Updates the value of `banned_pattern_mute_duration` for the specified chat.
    /// This way the users can override the default mute duration for the chat.
    #[instrument(skip(self))]
    pub(crate) async fn update_banned_pattern_mute_duration(
        &self,
        chat_id: ChatId,
        duration: Duration,
    ) -> Result {
        let query = sqlx::query!(
            "UPDATE tg_chats
            SET banned_pattern_mute_duration = $1
            WHERE id = $2",
            duration.try_into_db()?,
            chat_id.to_string(),
        );
        self.execute_or_chat_not_found(chat_id, query).await
    }

    /// Delete chat from tg_chats table
    #[instrument(skip(self))]
    pub(crate) async fn delete(&self, chat_id: ChatId) -> Result {
        let query = sqlx::query!("DELETE FROM tg_chats WHERE id = $1", chat_id.to_string(),);
        self.execute_or_chat_not_found(chat_id, query).await
    }

    async fn execute_or_chat_not_found(&self, chat_id: ChatId, query: PgQuery) -> Result {
        let affected = query.execute(&self.pool).await?.rows_affected();

        if affected > 1 {
            warn!(affected, "The query affected more than one row");
        }

        if affected == 1 {
            return Ok(());
        }

        Err(err_val!(UserError::ChatNotFound { chat_id }))
    }
}
