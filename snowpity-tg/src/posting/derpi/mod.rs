mod client;
mod db;

use super::{
    parse_with_regexes, Author, BlobId, BlobKind, BlobMeta, BlobSize, CachedBlob, ConfigTrait,
    DisplayInFileName, DistinctPostMetaTrait, MediaDimensions, ParseQueryResult, PostMeta,
    ServiceParams, ServiceTrait, TgFileMeta,
};
use crate::prelude::*;
use crate::Result;
use async_trait::async_trait;
use client::*;
use serde::Deserialize;
use std::collections::BTreeSet;

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
    const ENV_PREFIX: &'static str = "DERPI_";
}

pub(crate) struct Service {
    client: Client,
    db: db::MediaCacheRepo,
}

#[async_trait]
impl ServiceTrait for Service {
    const NAME: &str = "Derpibooru";

    type PostId = MediaId;
    type BlobId = ();
    type RequestId = MediaId;
    type Config = Config;
    type DistinctPostMeta = DistinctPostMeta;

    fn new(params: ServiceParams<Self::Config>) -> Self {
        Self {
            client: Client::new(params.config, params.http),
            db: db::MediaCacheRepo::new(params.db),
        }
    }

    fn parse_query(query: &str) -> ParseQueryResult<'_, Self::RequestId> {
        let (_, host, id) = parse_with_regexes!(
            query,
            r"(derpibooru.org(?:/images)?)/(\d+)",
            r"(derpicdn.net/img)/\d+/\d+/\d+/(\d+)",
            r"(derpicdn.net/img/(?:view|download))/\d+/\d+/\d+/(\d+)",
        )?;
        Some((host, id.parse().ok()?))
    }

    async fn get_post_meta(&self, request: Self::RequestId) -> Result<PostMeta> {
        let media = self
            .client
            .get_media(request)
            .instrument(info_span!("Fetching media meta from Derpibooru"))
            .await?;

        let authors = media
            .artists()
            .map(|artist| Author {
                web_url: client::artist_to_webpage_url(artist),
                name: artist.to_owned(),
            })
            .collect();

        let ratings = media.rating_tags().map(ToOwned::to_owned).collect();

        let dimensions = MediaDimensions {
            width: media.width,
            height: media.height,
        };

        use client::MimeType::*;
        let size = match media.mime_type {
            ImageJpeg | ImagePng | ImageSvgXml => BlobSize::approx_max_direct_photo_url(),
            ImageGif | VideoWebm => BlobSize::approx_max_direct_file_url(),
        };

        let blob = BlobMeta {
            id: BlobId::Derpi(()),
            dimensions,
            download_url: media.best_tg_url(),
            kind: media.mime_type.into(),
            // Sizes for images are ~good enough, although not always accurate,
            // but we don't know the size of MP4 equivalent for GIF or WEBM,
            // however those will often fit into the limit of uploading via direct URL.
            size,
        };

        Ok(PostMeta {
            id: media.id.into(),
            authors,
            web_url: media.id.to_webpage_url(),
            distinct: DistinctPostMeta { ratings }.into(),
            blobs: vec![blob],
        })
    }

    async fn get_cached_blobs(&self, request: Self::RequestId) -> Result<Vec<CachedBlob>> {
        Ok(self
            .db
            .get(request)
            .with_duration_log("Reading the cache from the database")
            .await?
            .map(|tg_file| CachedBlob {
                id: BlobId::Derpi(()),
                tg_file,
            }))
    }

    async fn set_cached_blob(
        &self,
        post: Self::PostId,
        (): Self::BlobId,
        tg_file: TgFileMeta,
    ) -> Result {
        self.db.set(post, tg_file).await
    }
}

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

impl DisplayInFileName for MediaId {
    fn display_in_file_name(&self) -> Option<String> {
        Some(self.to_string())
    }
}

impl From<client::MimeType> for BlobKind {
    fn from(value: client::MimeType) -> Self {
        match value {
            client::MimeType::ImageGif => BlobKind::AnimationMp4,
            client::MimeType::ImageJpeg => BlobKind::ImageJpeg,
            client::MimeType::ImagePng => BlobKind::ImagePng,
            client::MimeType::ImageSvgXml => BlobKind::ImageSvg,
            client::MimeType::VideoWebm => BlobKind::VideoMp4,
        }
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    #[test]
    fn smoke() {
        use crate::posting::tests::assert_parse_query as test;
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
