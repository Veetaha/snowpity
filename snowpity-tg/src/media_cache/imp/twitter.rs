use super::super::{
    service::Context, Artist, CachedMedia, FileSize, MediaDimensions, MediaHostSpecific, MediaId,
    MediaKind, MediaMeta, MAX_DIRECT_URL_FILE_SIZE, MB,
};
use crate::posting::twitter::{self, GetTweetResponse, TweetId};
use crate::observability::logging::prelude::*;
use crate::tg::MediaCacheError;
use crate::Result;

pub(crate) async fn get_media_meta(ctx: &Context, tweet_id: TweetId) -> Result<Vec<MediaMeta>> {
    let GetTweetResponse {
        author,
        media,
        tweet,
    } = ctx
        .media
        .twitter
        .get_tweet(tweet_id)
        .instrument(info_span!("Fetching media meta from Twitter"))
        .await?;

    let media: Vec<_> = media
        .into_iter()
        .map(|media| {
            Ok(MediaMeta {
                artists: <_>::from_iter([author.clone().into()]),
                web_url: author.tweet_url(tweet.id),
                download_url: media.best_tg_url()?,
                size: match media.kind {
                    // The size limits were taken from here:
                    // https://developer.twitter.com/en/docs/twitter-api/v1/media/upload-media/uploading-media/media-best-practices
                    twitter::MediaKind::Photo(_) => FileSize::Max(5 * MB),
                    twitter::MediaKind::AnimatedGif(_) => FileSize::Max(15 * MB),

                    // Technically the video can be up to 512MB, but optimisticaly
                    // we assume that most video are under 20MB to try uploading
                    // them via a direct URL to telegram first
                    twitter::MediaKind::Video(_) => FileSize::ApproxMax(MAX_DIRECT_URL_FILE_SIZE),
                },
                id: MediaId::Twitter(tweet.id, media.media_key),
                kind: (&media.kind).into(),
                // XXX: the dimensions are not always correct. They are for `orig`
                // representation, but we use `large` one. However this is a
                // good enough hint for aspect ratio checks in telegram uploads.
                // Either way orig's largest resolution 4096x4096 fits into the tg limits.
                dimensions: MediaDimensions {
                    width: media.width,
                    height: media.height,
                },
                host_specific: MediaHostSpecific::Twitter {
                    possibly_sensitive: tweet.possibly_sensitive,
                },
            })
        })
        .collect::<Result<_>>()?;

    if media.is_empty() {
        return Err(MediaCacheError::Twitter(TwitterMediaCacheError::MissingMedia).into());
    }

    Ok(media)
}

pub(crate) async fn get_cached_media(ctx: &Context, tweet_id: TweetId) -> Result<Vec<CachedMedia>> {
    let media = ctx
        .db
        .tg_media_cache
        .twitter
        .get(tweet_id)
        .with_duration_log("Reading the cache from the database")
        .await?
        .into_iter()
        .map(|cached| CachedMedia {
            id: MediaId::Twitter(cached.tweet_id, cached.media_key),
            tg_file: cached.tg_file,
        })
        .collect();

    Ok(media)
}

impl From<twitter::User> for Artist {
    fn from(user: twitter::User) -> Self {
        Self {
            web_url: user.web_url(),
            name: user.name,
        }
    }
}

impl From<&twitter::MediaKind> for MediaKind {
    fn from(kind: &twitter::MediaKind) -> Self {
        match kind {
            twitter::MediaKind::Photo(_) => MediaKind::ImageJpeg,
            twitter::MediaKind::AnimatedGif(_) => MediaKind::AnimationMp4,
            twitter::MediaKind::Video(_) => MediaKind::VideoMp4,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TwitterMediaCacheError {
    #[error("The tweet contains no media")]
    MissingMedia,
}
