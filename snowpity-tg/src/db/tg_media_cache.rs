use crate::prelude::*;
use crate::tg::TgFileType;
use crate::{derpi, Result};
use sqlx_bat::prelude::*;

pub(crate) struct TgMediaCacheRepo {
    db: sqlx::PgPool,
}

#[derive(Clone, Debug)]
pub(crate) struct CachedMedia {
    pub(crate) derpi_id: derpi::MediaId,
    pub(crate) tg_file_id: String,
    pub(crate) tg_file_type: TgFileType,
}

impl TgMediaCacheRepo {
    pub(crate) fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }

    #[metered_db]
    pub(crate) async fn set_derpi(&self, media: CachedMedia) -> Result {
        sqlx::query!(
            "insert into tg_media_cache (derpi_id, tg_file_id, tg_file_type)
            values ($1, $2, $3)",
            media.derpi_id.try_into_db()?,
            media.tg_file_id,
            media.tg_file_type.try_into_db()?,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    #[metered_db]
    pub(crate) async fn get_from_derpi(
        &self,
        derpi_id: derpi::MediaId,
    ) -> Result<Option<CachedMedia>> {
        sqlx::query!(
            "select tg_file_id, tg_file_type from tg_media_cache
            where derpi_id = $1",
            derpi_id.try_into_db()?,
        )
        .fetch_optional(&self.db)
        .await?
        .map(|record| {
            Ok(CachedMedia {
                derpi_id,
                tg_file_id: record.tg_file_id,
                tg_file_type: record.tg_file_type.try_into_app()?,
            })
        })
        .transpose()
    }
}
