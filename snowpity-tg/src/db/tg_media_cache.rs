use crate::db::conv;
use crate::util::prelude::*;
use crate::{derpi, Result};
use sea_orm::prelude::*;
use sea_orm::Set;

pub(crate) struct TgMediaCacheRepo {
    db: DatabaseConnection,
}

#[derive(Clone)]
pub(crate) struct CachedMedia {
    pub(crate) tg_file_id: String,
}

impl TgMediaCacheRepo {
    pub(crate) fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[instrument(skip(self))]
    pub(crate) async fn set_derpi(
        &self,
        media_id: derpi::MediaId,
        tg_file_id: &str,
    ) -> Result<CachedMedia> {
        let model = entities::tg_derpi_media_cache::ActiveModel {
            derpi_id: Set(conv::try_into_db(media_id.0)?),
            tg_file_id: Set(tg_file_id.to_owned()),
        };
        entities::TgDerpiMediaCache::insert(model)
            .exec(&self.db)
            .await?;

        let media = CachedMedia {
            tg_file_id: tg_file_id.to_owned(),
        };

        Ok(media)
    }

    pub(crate) async fn get_from_derpi(
        &self,
        derpi_id: derpi::MediaId,
    ) -> Result<Option<CachedMedia>> {
        let media = entities::TgDerpiMediaCache::find_by_id(conv::try_into_db(derpi_id.0)?)
            .one(&self.db)
            .await?
            .map(|media| CachedMedia {
                tg_file_id: media.tg_file_id,
            });
        Ok(media)
    }
}
