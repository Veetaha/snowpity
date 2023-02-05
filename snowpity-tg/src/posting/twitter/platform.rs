use crate::posting::platform::prelude::*;
use crate::posting::twitter::api::{self, MediaKey, TweetId};
use crate::posting::twitter::{db, Config};
use crate::prelude::*;
use crate::Result;
use async_trait::async_trait;

pub(crate) struct Platform {
    api: api::Client,
    db: db::BlobCacheRepo,
}

impl PlatformTypes for Platform {
    type PostId = TweetId;
    type BlobId = MediaKey;
    type RequestId = TweetId;
}

#[async_trait]
impl PlatformTrait for Platform {
    type Config = Config;

    const NAME: &'static str = "Twitter";

    fn new(params: PlatformParams<Config>) -> Self {
        Self {
            api: api::Client::new(params.config, params.http),
            db: db::BlobCacheRepo::new(params.db),
        }
    }

    fn parse_query(query: &str) -> ParseQueryResult<'_, TweetId> {
        // The regex was inspired by the one in the booru/scraper repository:
        // https://github.com/booru/scraper/blob/095771b28521b49ae67e30db2764406a68b74395/src/scraper/twitter.rs#L16
        let (_, host, id) = parse_with_regexes!(
            query,
            r"((?:(?:mobile\.)|vx)?twitter.com)/[A-Za-z\d_]+/status/(\d+)",
        )?;

        Some((host, id.parse().ok()?))
    }

    async fn get_post(&self, tweet_id: TweetId) -> Result<Post<Self>> {
        let api::GetTweetResponse {
            author,
            media,
            tweet,
        } = self
            .api
            .get_tweet(tweet_id)
            .instrument(info_span!("Fetching media meta from Twitter"))
            .await?;

        let blobs = media
            .into_iter()
            .map(|media| {
                let download_url = media.best_tg_url()?;
                let size = match media.kind {
                    // The size limits were taken from here:
                    // https://developer.twitter.com/en/docs/twitter-api/v1/media/upload-media/uploading-media/media-best-practices
                    api::MediaKind::Photo(_) => BlobSize::max_mb(5),
                    api::MediaKind::AnimatedGif(_) => BlobSize::max_mb(15),

                    // Technically the video can be up to 512MB, but optimisticaly
                    // we assume that most video are under 20MB to try uploading
                    // them via a direct URL to telegram first
                    api::MediaKind::Video(_) => BlobSize::approx_max_direct_file_url(),
                };

                Ok(Blob {
                    id: media.media_key,
                    kind: (&media.kind).into(),
                    // XXX: the dimensions are not always correct. They are for `orig`
                    // representation, but we use `large` one. However this is a
                    // good enough hint for aspect ratio checks in telegram uploads.
                    // Either way orig's largest resolution 4096x4096 fits into the tg limits.
                    dimensions: MediaDimensions {
                        width: media.width,
                        height: media.height,
                    },
                    size,
                    download_url,
                })
            })
            .collect::<Result<_>>()?;

        Ok(Post {
            base: BasePost {
                id: tweet.id,
                web_url: author.tweet_url(tweet.id),
                authors: <_>::from_iter([author.into()]),
                safety: SafetyRating::sfw_if(!tweet.possibly_sensitive),
            },
            blobs,
        })
    }

    async fn get_cached_blobs(&self, tweet: TweetId) -> Result<Vec<CachedBlobId<Self>>> {
        Ok(self
            .db
            .get(tweet)
            .with_duration_log("Reading the cache from the database")
            .await?
            .into_iter()
            .map(|record| CachedBlobId {
                id: record.media_key,
                tg_file: record.tg_file,
            })
            .collect())
    }

    async fn set_cached_blob(&self, tweet: TweetId, blob: CachedBlobId<Self>) -> Result {
        self.db.set(tweet, blob.id, blob.tg_file).await
    }
}

impl DisplayInFileNameViaToString for api::TweetId {}
impl DisplayInFileNameViaToString for api::MediaKey {}

impl From<api::User> for Author {
    fn from(user: api::User) -> Self {
        Self {
            web_url: user.web_url(),
            kind: None,
            name: user.name,
        }
    }
}

impl From<&api::MediaKind> for BlobKind {
    fn from(kind: &api::MediaKind) -> Self {
        match kind {
            api::MediaKind::Photo(_) => BlobKind::ImageJpeg,
            api::MediaKind::AnimatedGif(_) => BlobKind::AnimationMp4,
            api::MediaKind::Video(_) => BlobKind::VideoMp4,
        }
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    #[test]
    fn smoke() {
        use crate::posting::platform::tests::assert_parse_query as test;
        test(
            "https://twitter.com/NORDING34/status/1607191066318454791",
            expect!["twitter.com:Twitter(TweetId(1607191066318454791))"],
        );
        test(
            "https://vxtwitter.com/NORDING34/status/1607191066318454791",
            expect!["vxtwitter.com:Twitter(TweetId(1607191066318454791))"],
        );
        test(
            "https://mobile.twitter.com/NORDING34/status/1607191066318454791",
            expect!["mobile.twitter.com:Twitter(TweetId(1607191066318454791))"],
        );
    }
}
