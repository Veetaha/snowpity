use crate::db::conv;
use crate::util::prelude::*;
use crate::util::TgFileType;
use crate::{derpi, Result};
use sea_orm::prelude::*;
use sea_orm::Set;

pub(crate) struct TgMediaCacheRepo {
    db: DatabaseConnection,
}

#[derive(Clone, Debug)]
pub(crate) struct CachedMedia {
    pub(crate) derpi_id: derpi::MediaId,
    pub(crate) tg_file_id: String,
    pub(crate) tg_file_type: TgFileType,
}

impl TgMediaCacheRepo {
    pub(crate) fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[instrument(skip(self))]
    pub(crate) async fn set_derpi(&self, media: CachedMedia) -> Result {
        let model = entities::tg_derpi_media_cache::ActiveModel {
            derpi_id: Set(conv::try_into_db(media.derpi_id.0)?),
            tg_file_id: Set(media.tg_file_id),
            tg_file_type: Set(media.tg_file_type.into()),
        };
        entities::TgDerpiMediaCache::insert(model)
            .exec(&self.db)
            .await?;

        Ok(())
    }

    pub(crate) async fn get_from_derpi(
        &self,
        derpi_id: derpi::MediaId,
    ) -> Result<Option<CachedMedia>> {
        entities::TgDerpiMediaCache::find_by_id(conv::try_into_db(derpi_id.0)?)
            .one(&self.db)
            .await?
            .map(|media| {
                Ok(CachedMedia {
                    derpi_id,
                    tg_file_id: media.tg_file_id,
                    tg_file_type: conv::try_from_db(media.tg_file_type)?,
                })
            })
            .transpose()
    }
}
