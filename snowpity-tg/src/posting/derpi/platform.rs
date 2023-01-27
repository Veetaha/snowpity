use crate::posting::derpi::api::{self, MediaId};
use crate::posting::derpi::{db, Config};
use crate::posting::platform::prelude::*;
use crate::prelude::*;
use crate::Result;
use async_trait::async_trait;
use std::collections::BTreeSet;

pub(crate) struct Platform {
    client: api::Client,
    db: db::BlobCacheRepo,
}

impl PlatformTypes for Platform {
    type PostId = MediaId;
    type BlobId = ();
    type RequestId = MediaId;
    type DistinctPostMeta = DistinctPostMeta;
}

#[async_trait]
impl PlatformTrait for Platform {
    type Config = Config;

    const NAME: &'static str = "Derpibooru";

    fn new(params: PlatformParams<Config>) -> Self {
        Self {
            client: api::Client::new(params.config, params.http),
            db: db::BlobCacheRepo::new(params.db),
        }
    }

    fn parse_query(query: &str) -> ParseQueryResult<'_, MediaId> {
        let (_, host, id) = parse_with_regexes!(
            query,
            r"(derpibooru.org(?:/images)?)/(\d+)",
            r"(derpicdn.net/img)/\d+/\d+/\d+/(\d+)",
            r"(derpicdn.net/img/(?:view|download))/\d+/\d+/\d+/(\d+)",
        )?;
        Some((host, id.parse().ok()?))
    }

    async fn get_post(&self, media: MediaId) -> Result<Post<Self>> {
        let media = self
            .client
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

        let ratings = media.rating_tags().map(ToOwned::to_owned).collect();

        let dimensions = MediaDimensions {
            width: media.width,
            height: media.height,
        };

        use api::MimeType::*;
        let size = match media.mime_type {
            ImageJpeg | ImagePng | ImageSvgXml => BlobSize::approx_max_direct_photo_url(),
            ImageGif | VideoWebm => BlobSize::approx_max_direct_file_url(),
        };

        let blob = Blob {
            id: (),
            dimensions,
            download_url: media.best_tg_url(),
            kind: media.mime_type.into(),
            // Sizes for images are ~good enough, although not always accurate,
            // but we don't know the size of MP4 equivalent for GIF or WEBM,
            // however those will often fit into the limit of uploading via direct URL.
            size,
        };

        Ok(Post {
            base: BasePost {
                id: media.id,
                authors,
                web_url: media.id.to_webpage_url(),
                distinct: DistinctPostMeta { ratings },
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

#[derive(Clone)]
pub(crate) struct DistinctPostMeta {
    /// A set of tags `safe`, `suggestive`, `explicit`, etc.
    ratings: BTreeSet<String>,
}

impl DistinctPostMetaTrait for DistinctPostMeta {
    fn nsfw_ratings(&self) -> Vec<&str> {
        self.ratings
            .iter()
            .filter(|tag| *tag != "safe")
            .map(String::as_str)
            .collect()
    }
}

impl DisplayInFileNameViaToString for api::MediaId {}

impl From<api::MimeType> for BlobKind {
    fn from(value: api::MimeType) -> Self {
        match value {
            api::MimeType::ImageGif => BlobKind::AnimationMp4,
            api::MimeType::ImageJpeg => BlobKind::ImageJpeg,
            api::MimeType::ImagePng => BlobKind::ImagePng,
            api::MimeType::ImageSvgXml => BlobKind::ImageSvg,
            api::MimeType::VideoWebm => BlobKind::VideoMp4,
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
