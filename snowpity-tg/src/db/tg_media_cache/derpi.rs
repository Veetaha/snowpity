use crate::media_host::derpi;
use crate::prelude::*;
use crate::tg::TgFileMeta;
use crate::Result;
use sqlx_bat::prelude::*;

pub(crate) struct TgDerpiMediaCacheRepo {
    db: sqlx::PgPool,
}

impl TgDerpiMediaCacheRepo {
    pub(crate) fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }

    #[metered_db]
    pub(crate) async fn set(&self, derpi_id: derpi::MediaId, tg_file: TgFileMeta) -> Result {
        sqlx::query!(
            "insert into tg_derpi_media_cache (derpi_id, tg_file_id, tg_file_kind)
            values ($1, $2, $3)",
            derpi_id.try_into_db()?,
            tg_file.id,
            tg_file.kind.try_into_db()?,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    #[metered_db]
    pub(crate) async fn get(&self, derpi_id: derpi::MediaId) -> Result<Option<TgFileMeta>> {
        sqlx::query!(
            "select tg_file_id, tg_file_kind from tg_derpi_media_cache
            where derpi_id = $1",
            derpi_id.try_into_db()?,
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
