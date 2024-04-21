use crate::posting::{derpilike::api::MediaId, TgFileMeta};
use crate::prelude::*;
use crate::Result;
use sqlx_bat::{prelude::*, TryIntoDb};

pub(crate) struct BlobCacheRepo {
    db: sqlx::PgPool,
    table_name: &'static str,
}

#[derive(sqlx::FromRow)]
struct Record {
    tg_file_id: String,
    tg_file_kind: i16,
}

impl BlobCacheRepo {
    pub(crate) fn new(db: sqlx::PgPool, table_name: &'static str) -> Self {
        Self { db, table_name }
    }

    #[metered_db]
    pub(crate) async fn set(&self, media_id: MediaId, tg_file: TgFileMeta) -> Result {
        let query = format!(
            "insert into tg_{}_blob_cache (media_id, tg_file_id, tg_file_kind)
            values ($1, $2, $3)",
            self.table_name
        );

        sqlx::query(&query)
            .bind(media_id.try_into_db()?)
            .bind(tg_file.id)
            .bind(tg_file.kind.try_into_db()?)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    #[metered_db]
    pub(crate) async fn get(&self, media_id: MediaId) -> Result<Option<TgFileMeta>> {
        let query = format!(
            "select tg_file_id, tg_file_kind from tg_{}_blob_cache
            where media_id = $1",
            self.table_name
        );

        sqlx::query_as(&query)
            .bind(media_id.try_into_db()?)
            .fetch_optional(&self.db)
            .await?
            .map(
                |Record {
                     tg_file_id,
                     tg_file_kind,
                 }| {
                    Ok(TgFileMeta {
                        id: tg_file_id,
                        kind: tg_file_kind.try_into_app()?,
                    })
                },
            )
            .transpose()
    }
}
