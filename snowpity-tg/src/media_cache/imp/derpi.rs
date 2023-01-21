use crate::posting::derpi;
use crate::observability::logging::prelude::*;
use crate::tg::media_cache::{
    service::Context, Artist, CachedMedia, FileSize, MediaDimensions, MediaHostSpecific, MediaKind,
    MediaMeta, MAX_DIRECT_URL_FILE_SIZE, MAX_DIRECT_URL_PHOTO_SIZE,
};
use crate::Result;

pub(crate) async fn get_media_meta(ctx: &Context, media_id: derpi::MediaId) -> Result<MediaMeta> {

}
