use crate::media_host::twitter::{self, TweetId};
use crate::prelude::*;
use crate::tg::TgFileMeta;
use crate::Result;
use sqlx_bat::prelude::*;

pub(crate) struct TgTwitterMediaCacheRepo {
    db: sqlx::PgPool,
}

impl TgTwitterMediaCacheRepo {
    pub(crate) fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }

    #[metered_db]
    pub(crate) async fn set(
        &self,
        tweet_id: TweetId,
        media: Vec<twitter::MediaKey>,
        tg_file: TgFileMeta,
    ) -> Result {
        sqlx::query!(
            "insert into tg_media_cache (derpi_id, tg_file_id, tg_file_type)
            values ($1, $2, $3)",
            media.derpi_id.try_into_db()?,
            media.tg_file.id,
            media.tg_file.kind.try_into_db()?,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    #[metered_db]
    pub(crate) async fn get(&self, tweet_id: TweetId) -> Result<Vec<TgFileMeta>> {
        sqlx::query!(
            "select tg_file_id, tg_file_type from tg_media_cache
            where derpi_id = $1",
            derpi_id.try_into_db()?,
        )
        .fetch_optional(&self.db)
        .await?
        .map(|record| {
            Ok(TgFileMeta {
                id: record.tg_file_id,
                kind: record.tg_file_type.try_into_app()?,
            })
        })
        .transpose()
    }
}
