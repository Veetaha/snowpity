use super::super::{Artist, CachedMedia, Context, MediaHostSpecific, MediaMeta};
use crate::media_host::derpi;
use crate::observability::logging::prelude::*;
use crate::Result;
use futures::prelude::*;

pub(crate) async fn get_media_meta(ctx: &Context, media_id: derpi::MediaId) -> Result<MediaMeta> {
    ctx.media
        .derpi
        .get_media(media_id)
        .instrument(info_span!("Fetching media meta from Derpibooru"))
        .map_ok(Into::into)
        .await
}

pub(crate) async fn get_cached_media(
    ctx: &Context,
    media_id: derpi::MediaId,
) -> Result<Option<CachedMedia>> {
    Ok(ctx
        .db
        .tg_media_cache_derpi
        .get(media_id)
        .with_duration_log("Reading the cache from the database")
        .await?
        .map(|cached| CachedMedia {
            id: media_id.into(),
            tg_file_id: cached.id,
            tg_file_type: cached.kind,
        }))
}

impl From<derpi::Media> for MediaMeta {
    fn from(media: derpi::Media) -> Self {
        Self {
            id: media.id.into(),
            artists: media
                .artists()
                .map(|artist| Artist {
                    link: derpi::artist_to_webpage_url(artist),
                    name: artist.to_owned(),
                })
                .collect(),
            web_url: media.id.to_webpage_url(),
            host_specific: MediaHostSpecific::Derpibooru {
                ratings: media.rating_tags().map(ToOwned::to_owned).collect(),
            },
        }
    }
}
