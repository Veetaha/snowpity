use crate::posting::platform::prelude::*;
use crate::posting::twitter::api::{self, MediaKey, TweetId};
use crate::posting::twitter::{db, Config};
use crate::prelude::*;
use crate::Result;
use async_trait::async_trait;
use url::Url;

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
            api: api::Client::new(params.config),
            db: db::BlobCacheRepo::new(params.db),
        }
    }

    fn parse_query(query: &str) -> ParseQueryResult<TweetId> {
        // The regex was inspired by the one in the booru/scraper repository:
        // https://github.com/booru/scraper/blob/095771b28521b49ae67e30db2764406a68b74395/src/scraper/twitter.rs#L16
        let (_, host, id) = x(
            query,
            r"((?:mobile\.|vx)?(?:twitter|x)\.com)/[A-Za-z\d_]+/status/(\d+)",
        )?;

        Some((host.into(), id.parse().ok()?))
    }

    async fn get_post(&self, tweet_id: TweetId) -> Result<Post<Self>> {
        let tweet = self
            .api
            .get_tweet(tweet_id)
            .instrument(info_span!("Fetching media meta from Twitter"))
            .await?;

        let web_url = tweet.tweet_url(tweet.id);
        let author = Author {
            web_url: tweet.author_web_url(),
            kind: None,
            name: tweet.name,
        };

        // TODO: the dimensions are not available in the API right now,
        // however this we could potentially modify the twitter-scraper
        // code to return them. The author of that go library accepts PRs
        //
        // The size limits were taken from here:
        // https://developer.twitter.com/en/docs/twitter-api/v1/media/upload-media/uploading-media/media-best-practices

        let blobs = tweet.photos.into_iter().map(|media| {
            (
                media.id,
                BlobRepr {
                    kind: BlobKind::ImageJpeg,
                    size: BlobSize::max_mb(5),
                    download_url: best_tg_url_for_photo(media.url),
                    dimensions: None,
                },
            )
        });

        let gifs = tweet.gifs.into_iter().map(|media| {
            (
                media.id,
                BlobRepr {
                    kind: BlobKind::AnimationMp4,
                    download_url: media.url,
                    size: BlobSize::max_mb(15),
                    dimensions: None,
                },
            )
        });

        let videos = tweet.videos.into_iter().map(|media| {
            (
                media.id,
                BlobRepr {
                    kind: BlobKind::VideoMp4,
                    // Technically the video can be up to 512MB
                    size: BlobSize::Unknown,
                    download_url: media.url,
                    dimensions: None,
                },
            )
        });

        let blobs = blobs
            .chain(gifs)
            .chain(videos)
            .map_collect(|(id, repr)| MultiBlob {
                id: MediaKey::from_raw(id),
                repr: vec![repr],
            });

        Ok(Post {
            base: BasePost {
                id: tweet.id,
                web_url,
                authors: <_>::from_iter([author]),
                safety: SafetyRating::sfw_if(!tweet.sensitive_content),
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
            .map_collect(|record| CachedBlobId {
                id: record.media_key,
                tg_file: record.tg_file,
            }))
    }

    async fn set_cached_blob(&self, tweet: TweetId, blob: CachedBlobId<Self>) -> Result {
        self.db.set(tweet, blob.id, blob.tg_file).await
    }
}

impl DisplayInFileNameViaToString for api::TweetId {}
impl DisplayInFileNameViaToString for api::MediaKey {}

/// URL of the media that best suits Telegram.
///
/// The images will fit into `4096x4096` bounding box.
/// This doesn't however guarantee the images will have top-notch quality (see [wiki]).
///
/// For videos and gifs the format is `video/mp4` with the highest bitrate.
///
/// Media URL formatting is described in twitter [API v1.1 docs].
/// See also this [community thread] that refers to the same docs.
///
/// [API v1.1 docs]: https://developer.twitter.com/en/docs/twitter-api/v1/data-dictionary/object-model/entities#photo_format
/// [wiki]: https://github.com/Veetaha/snowpity/wiki/Telegram-images-compression
/// [community thread]: https://twittercommunity.com/t/retrieving-full-size-images-media-fields-url-points-to-resized-version/160494/2
fn best_tg_url_for_photo(mut url: Url) -> Url {
    url.query_pairs_mut().append_pair("name", "orig");
    url
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
            "https://x.com/NORDING34/status/1607191066318454791",
            expect!["x.com:Twitter(TweetId(1607191066318454791))"],
        );
        test(
            "https://vxtwitter.com/NORDING34/status/1607191066318454791",
            expect!["vxtwitter.com:Twitter(TweetId(1607191066318454791))"],
        );
        test(
            "https://mobile.twitter.com/NORDING34/status/1607191066318454791",
            expect!["mobile.twitter.com:Twitter(TweetId(1607191066318454791))"],
        );
        test(
            "https://mobile.x.com/NORDING34/status/1607191066318454791",
            expect!["mobile.x.com:Twitter(TweetId(1607191066318454791))"],
        );
    }
}
