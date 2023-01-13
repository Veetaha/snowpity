use crate::media_host::derpi;
use crate::observability::logging::prelude::*;
use crate::tg::media_cache::{
    service::Context, Artist, CachedMedia, FileSize, MediaDimensions, MediaHostSpecific, MediaKind,
    MediaMeta, MAX_DIRECT_URL_FILE_SIZE, MAX_DIRECT_URL_PHOTO_SIZE,
};
use crate::Result;

pub(crate) async fn get_media_meta(ctx: &Context, media_id: derpi::MediaId) -> Result<MediaMeta> {
    let media = ctx
        .media
        .derpi
        .get_media(media_id)
        .instrument(info_span!("Fetching media meta from Derpibooru"))
        .await?;

    let artists = media
        .artists()
        .map(|artist| Artist {
            web_url: derpi::artist_to_webpage_url(artist),
            name: artist.to_owned(),
        })
        .collect();

    let ratings = media.rating_tags().map(ToOwned::to_owned).collect();

    let dimensions = MediaDimensions {
        width: media.width,
        height: media.height,
    };

    use derpi::MimeType::*;

    Ok(MediaMeta {
        id: media.id.into(),
        artists,
        web_url: media.id.to_webpage_url(),
        host_specific: MediaHostSpecific::Derpibooru { ratings },
        dimensions,
        download_url: media.best_tg_url(),
        kind: media.mime_type.into(),
        // Sizes for images are ~good enough, although not always accurate,
        // but we don't know the size of MP4 equivalent for GIF or WEBM,
        // however those will often fit into the limit of uploading via direct URL.
        size: match media.mime_type {
            ImageJpeg | ImagePng | ImageSvgXml => FileSize::ApproxMax(MAX_DIRECT_URL_PHOTO_SIZE),
            ImageGif | VideoWebm => FileSize::Approx(MAX_DIRECT_URL_FILE_SIZE),
        },
    })
}

pub(crate) async fn get_cached_media(
    ctx: &Context,
    media_id: derpi::MediaId,
) -> Result<Option<CachedMedia>> {
    Ok(ctx
        .db
        .tg_media_cache
        .derpi
        .get(media_id)
        .with_duration_log("Reading the cache from the database")
        .await?
        .map(|tg_file| CachedMedia {
            id: media_id.into(),
            tg_file,
        }))
}

impl From<derpi::MimeType> for MediaKind {
    fn from(value: derpi::MimeType) -> Self {
        match value {
            derpi::MimeType::ImageGif => MediaKind::AnimationMp4,
            derpi::MimeType::ImageJpeg => MediaKind::ImageJpeg,
            derpi::MimeType::ImagePng => MediaKind::ImagePng,
            derpi::MimeType::ImageSvgXml => MediaKind::ImageSvg,
            derpi::MimeType::VideoWebm => MediaKind::VideoMp4,
        }
    }
}
