use crate::db::conv;
use crate::util::prelude::*;
use crate::{derpi, Result};
use sea_orm::prelude::*;
use sea_orm::Set;

pub(crate) struct TgMediaCacheRepo {
    db: DatabaseConnection,
}

impl TgMediaCacheRepo {
    pub(crate) fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[instrument(skip(self))]
    pub(crate) async fn create_derpi(&self, derpi_id: derpi::MediaId, tg_file_id: &str) -> Result {
        let media = entities::tg_derpi_media_cache::ActiveModel {
            derpi_id: Set(conv::try_into_db(derpi_id.0)?),
            tg_file_id: Set(tg_file_id.to_owned()),
        };
        entities::TgDerpiMediaCache::insert(media)
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_from_derpi(&self, derpi_id: derpi::MediaId) -> Result<Option<String>> {
        let media = entities::TgDerpiMediaCache::find_by_id(conv::try_into_db(derpi_id.0)?)
            .one(&self.db)
            .await?;
        Ok(media.map(|media| media.tg_file_id))
    }
}
