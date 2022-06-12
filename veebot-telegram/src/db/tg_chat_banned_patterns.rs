use crate::db::db_constraints;
use crate::util::prelude::*;
use crate::Result;
use crate::{err_val, UserError};
use chrono::prelude::*;
use futures::prelude::*;
use regex::Regex;
use teloxide::types::{ChatId, UserId};
use tracing::instrument;

#[derive(Debug)]
pub(crate) struct BannedPattern {
    pub(crate) pattern: Regex,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) created_by: UserId,
}

pub(crate) struct TgChatBannedPatternsRepo {
    pool: sqlx::PgPool,
}

impl TgChatBannedPatternsRepo {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self))]
    pub(crate) async fn create(
        &self,
        chat_id: ChatId,
        pattern: &Regex,
        created_by: UserId,
    ) -> Result {
        let query = sqlx::query!(
            "INSERT INTO tg_chat_banned_patterns (tg_chat_id, pattern, created_by)
            VALUES ($1, $2, $3)",
            chat_id.to_string(),
            pattern.as_str(),
            created_by.to_string(),
        );

        query.execute(&self.pool).await.map_err(|err| {
            if err.is_constraint_violation(db_constraints::TG_CHAT_AND_PATTERN_COMPOSITE_PK) {
                return err_val!(UserError::BannedPatternAlreadyExists {
                    pattern: pattern.clone()
                });
            }
            err.into()
        })?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub(crate) fn get_all_by_chat_id(
        &self,
        chat_id: ChatId,
    ) -> impl Stream<Item = Result<BannedPattern>> + '_ {
        let query = sqlx::query!(
            "SELECT pattern, created_at, created_by
            FROM tg_chat_banned_patterns WHERE tg_chat_id = $1",
            chat_id.to_string(),
        );

        query
            .fetch(&self.pool)
            .map_ok(|record| {
                let pattern = Regex::new(&record.pattern).unwrap();
                BannedPattern {
                    pattern,
                    created_at: record.created_at,
                    created_by: UserId(record.created_by.parse().unwrap()),
                }
            })
            .err_into()
    }

    #[instrument(skip(self))]
    pub(crate) async fn delete(&self, chat_id: ChatId, pattern: &Regex) -> Result {
        let query = sqlx::query!(
            "DELETE FROM tg_chat_banned_patterns
            WHERE tg_chat_id = $1 AND pattern = $2",
            chat_id.to_string(),
            pattern.as_str(),
        );

        if query.execute(&self.pool).await?.rows_affected() == 0 {
            return Err(err_val!(UserError::BannedPatternNotFound {
                pattern: pattern.clone()
            }));
        }

        Ok(())
    }
}
