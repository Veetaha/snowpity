use crate::posting::{derpibooru::api::MediaId, TgFileMeta};
use crate::prelude::*;
use crate::Result;
use sqlx_bat::prelude::*;

pub(crate) struct BlobCacheRepo {
    db: sqlx::PgPool,
}

impl BlobCacheRepo {
    pub(crate) fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }

    #[metered_db]
    pub(crate) async fn set(&self, derpibooru_id: MediaId, tg_file: TgFileMeta) -> Result {
        sqlx::query!(
            "insert into tg_derpibooru_blob_cache (derpibooru_id, tg_file_id, tg_file_kind)
            values ($1, $2, $3)",
            derpibooru_id.try_into_db()?,
            tg_file.id,
            tg_file.kind.try_into_db()?,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    #[metered_db]
    pub(crate) async fn get(&self, derpibooru_id: MediaId) -> Result<Option<TgFileMeta>> {
        sqlx::query!(
            "select tg_file_id, tg_file_kind from tg_derpibooru_blob_cache
            where derpibooru_id = $1",
            derpibooru_id.try_into_db()?,
        )
        .fetch_optional(&self.db)
        .await?
        .map(|record| {
            Ok(TgFileMeta {
                id: record.tg_file_id,
                kind: record.tg_file_kind.try_into_app()?,
            })
        })
        .transpose()
    }
}
