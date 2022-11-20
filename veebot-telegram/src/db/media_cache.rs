use crate::db::db_constraints;
use crate::util::prelude::*;
use crate::{Result, derpi};

pub(crate) struct MediaCacheRepo {
    pool: sqlx::PgPool,
}

impl MediaCacheRepo {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self))]
    pub(crate) async fn create(&self, media_id: derpi::MediaId, tg_file_id: &str) -> Result {
        let query = sqlx::query!(
            "insert into tg_derpi_media_cache (media_id, tg_file_id) values ($1, $2)",
            media_id.try_into_db()?,
            tg_file_id,
        );

        let result = query.execute(&self.pool).await;
        let Err(err) = result else {
            return Ok(());
        };

        if err.is_constraint_violation(db_constraints::TG_DERPI_MEDIA_CACHE_MEDIA_ID_PK) {
            warn!("Media cache entry already exists");
            return Ok(());
        }

        Err(err.into())
    }

    pub(crate) async fn get_derpi_tg_file_id(&self, media_id: derpi::MediaId) -> Result<Option<String>> {
        let query = sqlx::query!(
            "select tg_file_id from tg_derpi_media_cache where media_id = $1",
            media_id.try_into_db_imp()?,
        );

        let row = query.fetch_optional(&self.pool).await?;

        Ok(row.map(|row| row.tg_file_id))
    }
}
