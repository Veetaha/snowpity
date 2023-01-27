use crate::posting::twitter::api::{MediaKey, TweetId};
use crate::posting::TgFileMeta;
use crate::prelude::*;
use crate::Result;
use futures::prelude::*;
use sqlx_bat::prelude::*;

pub(crate) struct BlobCacheRepo {
    db: sqlx::PgPool,
}

pub(crate) struct CachedMediaRecord {
    // pub(crate) tweet_id: TweetId,
    pub(crate) media_key: MediaKey,
    pub(crate) tg_file: TgFileMeta,
}

impl BlobCacheRepo {
    pub(crate) fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }

    #[metered_db]
    pub(crate) async fn set(
        &self,
        tweet_id: TweetId,
        media_key: MediaKey,
        tg_file: TgFileMeta,
    ) -> Result {
        sqlx::query!(
            "insert into tg_twitter_blob_cache (
                tweet_id,
                media_key,
                tg_file_id,
                tg_file_kind
            )
            values ($1, $2, $3, $4)",
            tweet_id.try_into_db()?,
            media_key.try_into_db()?,
            tg_file.id,
            tg_file.kind.try_into_db()?,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    #[metered_db]
    pub(crate) async fn get(&self, tweet_id: TweetId) -> Result<Vec<CachedMediaRecord>> {
        sqlx::query!(
            "select media_key, tg_file_id, tg_file_kind
            from tg_twitter_blob_cache
            where tweet_id = $1",
            tweet_id.try_into_db()?,
        )
        .fetch(&self.db)
        .err_into()
        .and_then(|record| async move {
            Ok::<_, crate::Error>(CachedMediaRecord {
                media_key: record.media_key.try_into_app()?,
                tg_file: TgFileMeta {
                    id: record.tg_file_id,
                    kind: record.tg_file_kind.try_into_app()?,
                },
            })
        })
        .try_collect()
        .await
    }
}
