use crate::posting::derpibooru::api::{self, MediaId};
use crate::posting::derpibooru::{db, Config};
use crate::posting::platform::prelude::*;
use crate::prelude::*;
use crate::Result;
use async_trait::async_trait;
use reqwest::Url;

pub(crate) struct Platform {
    api: api::Client,
    db: db::BlobCacheRepo,
}

impl PlatformTypes for Platform {
    type PostId = MediaId;
    type BlobId = ();
    type RequestId = MediaId;
}

#[async_trait]
impl PlatformTrait for Platform {
    type Config = Config;

    const NAME: &'static str = "Derpibooru";

    fn new(params: PlatformParams<Config>) -> Self {
        Self {
            api: api::Client::new(params.config, params.http),
            db: db::BlobCacheRepo::new(params.db),
        }
    }

    fn parse_query(query: &str) -> ParseQueryResult<MediaId> {
        let (_, host, id) = parse_with_regexes!(
            query,
            r"(derpibooru.org(?:/images)?)/(\d+)",
            r"(derpicdn.net/img)/\d+/\d+/\d+/(\d+)",
            r"(derpicdn.net/img/(?:view|download))/\d+/\d+/\d+/(\d+)",
        )?;
        Some((host.into(), id.parse().ok()?))
    }

    async fn get_post(&self, media: MediaId) -> Result<Post<Self>> {
        let media = self
            .api
            .get_media(media)
            .instrument(info_span!("Fetching media meta from Derpibooru"))
            .await?;

        let authors = media
            .authors()
            .map(|author| Author {
                web_url: author.web_url(),
                kind: match author.kind {
                    api::AuthorKind::Artist => None,
                    api::AuthorKind::Editor => Some(AuthorKind::Editor),
                },
                name: author.name,
            })
            .collect();

        let safety = media.safety_rating_tags().map(ToOwned::to_owned).collect();
        let safety = if safety == ["safe"] {
            SafetyRating::Sfw
        } else {
            SafetyRating::Nsfw { kinds: safety }
        };

        let dimensions = MediaDimensions {
            width: media.width,
            height: media.height,
        };

        let repr = best_tg_reprs(&media)
            .into_iter()
            .map(|(download_url, kind)| {
                BlobRepr {
                    dimensions,
                    download_url,
                    kind,
                    // Sizes for images are ~good enough, although not always accurate,
                    // but we don't know the size of MP4 equivalent for GIF or WEBM,
                    // however those will often fit into the limit of uploading via direct URL.
                    // Anyway, this is all not precise, so be it this way for now.
                    size: BlobSize::Unknown,
                }
            })
            .collect();

        let blob = MultiBlob { id: (), repr };

        Ok(Post {
            base: BasePost {
                id: media.id,
                authors,
                web_url: media.id.to_webpage_url(),
                safety,
            },
            blobs: vec![blob],
        })
    }

    async fn get_cached_blobs(&self, media: MediaId) -> Result<Vec<CachedBlobId<Self>>> {
        Ok(Vec::from_iter(
            self.db
                .get(media)
                .with_duration_log("Reading the cache from the database")
                .await?
                .map(CachedBlobId::with_tg_file),
        ))
    }

    async fn set_cached_blob(&self, media: MediaId, blob: CachedBlobId<Self>) -> Result {
        self.db.set(media, blob.tg_file).await
    }
}

impl DisplayInFileNameViaToString for api::MediaId {}

/// URL of the media that best suits Telegram.
///
/// Right now this is just the `view_url`, i.e. the original image representation.
/// Best would be if derpibooru could generate the representation of an image for
/// 2560x2560 pixels, but the biggest non-original representation is 1280x1024,
/// according to philomena's [sources].
///
/// This doesn't however guarantee the images will have top-notch quality (see [wiki]).
/// The GIFs don't use the `passthrough` flag when they are converted to MP4,
/// which means the FPS of the MP4 may be lower than the original GIF, so we
/// are re-generating the MP4 on the fly ourselves.
///
/// [wiki]: https://github.com/Veetaha/snowpity/wiki/Telegram-images-compression
/// [sources]: https://github.com/philomena-dev/philomena/blob/743699c6afe38b20b23f866c2c1a590c86d6095e/lib/philomena/images/thumbnailer.ex#L16-L24
fn best_tg_reprs(media: &api::Media) -> Vec<(Url, BlobKind)> {
    let blob_kind = match media.mime_type {
        api::MimeType::ImageJpeg => BlobKind::ImageJpeg,
        api::MimeType::ImagePng => BlobKind::ImagePng,
        api::MimeType::ImageSvgXml => BlobKind::ImageSvg,
        api::MimeType::ImageGif => {
            return vec![
                (media.unwrap_mp4_url(), BlobKind::AnimationMp4),
                (media.view_url.clone(), BlobKind::AnimationGif),
            ]
        }
        api::MimeType::VideoWebm => return vec![(media.unwrap_mp4_url(), BlobKind::VideoMp4)],
    };
    vec![(media.view_url.clone(), blob_kind)]
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    #[test]
    fn smoke() {
        use crate::posting::platform::tests::assert_parse_query as test;
        test(
            "derpibooru.org/123/",
            expect!["derpibooru.org:Derpibooru(MediaId(123))"],
        );
        test(
            "derpibooru.org/123",
            expect!["derpibooru.org:Derpibooru(MediaId(123))"],
        );
        test(
            "derpibooru.org/images/123",
            expect!["derpibooru.org/images:Derpibooru(MediaId(123))"],
        );
        test(
            "derpibooru.org/images/123/",
            expect!["derpibooru.org/images:Derpibooru(MediaId(123))"],
        );
        test(
            "https://derpicdn.net/img/2022/12/17/3008328/large.jpg",
            expect!["derpicdn.net/img:Derpibooru(MediaId(3008328))"],
        );
        test(
            "https://derpicdn.net/img/view/2022/12/17/3008328.jpg",
            expect!["derpicdn.net/img/view:Derpibooru(MediaId(3008328))"],
        );
        test(
            "https://derpicdn.net/img/download/2022/12/28/3015836__safe_artist-colon-shadowreindeer_foo.jpg",
            expect!["derpicdn.net/img/download:Derpibooru(MediaId(3015836))"]
        );
    }
}
