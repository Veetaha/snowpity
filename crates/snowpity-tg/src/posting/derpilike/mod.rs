use crate::posting::platform::prelude::*;
use crate::prelude::*;
use crate::Result;

use itertools::Either;
use reqwest::Url;
use serde::Deserialize;

use self::api::MediaId;

mod api;
mod db;

pub(crate) mod derpibooru;
pub(crate) mod ponerpics;

#[derive(Clone, Deserialize)]
pub(crate) struct Config {
    // Derpibooru doesn't require an API key for read-only requests.
    // The rate limiting is also the same for both anonymous and authenticated requests,
    // therefore we don't really need an API key
    //
    // This was confirmed by the Derpibooru staff in discord:
    // https://discord.com/channels/430829008402251796/438029140659142657/1059492359122989146
    //
    // This config struct exists here, just in case some day we do need to use an API key,
    // or want any other config options.
    //
    // api_key: String,
}

impl ConfigTrait for Config {
    const ENV_PREFIX: &'static str = "DERPIBOORU_";
}

struct Derpitools {
    api: api::Client,
    db: db::BlobCacheRepo,
    platform: DerpiPlatformKind,
}

impl Derpitools {
    async fn get_post<Platform: PlatformTrait<BlobId = (), PostId = MediaId>>(
        &self,
        media: MediaId,
    ) -> Result<Post<Platform>> {
        let media = self
            .api
            .get_media(media)
            .instrument(info_span!(
                "fetching_media",
                platform = %self.platform
            ))
            .await?;

        let authors = media.authors().map_collect(|author| Author {
            web_url: author.web_url(),
            kind: match author.kind {
                api::AuthorKind::Artist => None,
                api::AuthorKind::Editor => Some(AuthorKind::Editor),
                api::AuthorKind::Prompter => Some(AuthorKind::Prompter),
            },
            name: author.name,
        });

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
        let repr = best_tg_reprs(&media).map_collect(|(download_url, kind)| {
            BlobRepr {
                dimensions: Some(dimensions),
                download_url,
                kind,
                // Sizes for images are ~good enough, although not always accurate,
                // but we don't know the size of MP4 equivalent for GIF or WEBM,
                // however those will often fit into the limit of uploading via direct URL.
                // Anyway, this is all not precise, so be it this way for now.
                size: BlobSize::Unknown,
            }
        });

        let blob = MultiBlob { id: (), repr };

        Ok(Post {
            base: BasePost {
                id: media.id,
                authors,
                web_url: media.id.to_webpage_url(self.platform),
                safety,
            },
            blobs: vec![blob],
        })
    }

    async fn get_cached_blobs<Platform: PlatformTrait<BlobId = ()>>(
        &self,
        media_id: MediaId,
    ) -> Result<Vec<CachedBlobId<Platform>>> {
        Ok(Vec::from_iter(
            self.db
                .get(media_id)
                .with_duration_log("Reading the cache from the database")
                .await?
                .map(CachedBlobId::with_tg_file),
        ))
    }

    async fn set_cached_blob<Platform: PlatformTrait<BlobId = ()>>(
        &self,
        media_id: MediaId,
        blob: CachedBlobId<Platform>,
    ) -> Result {
        self.db.set(media_id, blob.tg_file).await
    }
}

#[derive(strum::Display, strum::IntoStaticStr, Debug, Clone, Copy)]
pub(crate) enum DerpiPlatformKind {
    Derpibooru,
    Ponerpics,
}

impl DerpiPlatformKind {
    pub(crate) fn db_table_name(self) -> &'static str {
        match self {
            DerpiPlatformKind::Derpibooru => "derpibooru",
            DerpiPlatformKind::Ponerpics => "ponerpics",
        }
    }

    pub(crate) fn base_url(self) -> Url {
        let url = match self {
            DerpiPlatformKind::Derpibooru => "https://derpibooru.org",
            DerpiPlatformKind::Ponerpics => "https://ponerpics.org",
        };
        url.parse().unwrap_or_else(|err| {
            panic!(
                "Failed to parse base URL.\n\
                url: {url:?}\n\
                platform: {self:#?}\n\
                Error: {err:#?}",
            );
        })
    }

    pub(crate) fn url(self, segments: impl IntoIterator<Item = impl AsRef<str>>) -> Url {
        let mut url = self.base_url();
        url.path_segments_mut()
            .unwrap_or_else(|()| {
                panic!(
                    "Base URL can not be a base\n\
                    url: {}\n\
                    platform: {self:#?}",
                    self.base_url(),
                )
            })
            .extend(segments);

        url
    }

    pub(crate) fn api_url(self, segments: impl IntoIterator<Item = impl AsRef<str>>) -> Url {
        let base = ["api", "v1", "json"].into_iter().map(Either::Left);
        let segments = segments.into_iter().map(Either::Right);

        self.url(itertools::chain(base, segments))
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
    match media.mime_type {
        api::MimeType::ImageJpeg => vec![(media.view_url.clone(), BlobKind::ImageJpeg)],
        api::MimeType::ImagePng => vec![(media.view_url.clone(), BlobKind::ImagePng)],
        api::MimeType::ImageSvgXml => vec![(media.view_url.clone(), BlobKind::ImageSvg)],
        api::MimeType::ImageGif => {
            vec![
                // First of all try to get an existing MP4 representation for the GIF
                (media.unwrap_mp4_url(), BlobKind::AnimationMp4),
                // If there is no MP4 representation, then generate it on the fly
                // from the original GIF file
                (media.view_url.clone(), BlobKind::AnimationGif),
            ]
        }
        api::MimeType::VideoWebm => vec![(media.unwrap_mp4_url(), BlobKind::VideoMp4)],
    }
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
