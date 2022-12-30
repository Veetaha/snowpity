use super::super::{Artist, Context, MediaHostSpecific, MediaMeta, CachedMedia};
use crate::media_host::{derpi, twitter};
use crate::observability::logging::prelude::*;
use crate::Result;
use futures::prelude::*;
use crate::media_host::twitter::TweetId;

pub(crate) async fn get_media_meta(ctx: &Context, media_id: TweetId) -> Result<Vec<MediaMeta>> {
    todo!()
    // ctx.media
    //     .derpi
    //     .get_media(media_id)
    //     .instrument(info_span!("Fetching media meta from Derpibooru"))
    //     .map_ok(Into::into)
    //     .await
}

pub(crate) async fn get_cached_media(ctx: &Context, media_id: TweetId) -> Result<Vec<CachedMedia>> {
    todo!()
    // ctx.db
    //     .tg_media_cache
    //     .get_from_derpi(media_id)
    //     .with_duration_log("Reading the cache from the database")
    //     .await
}
