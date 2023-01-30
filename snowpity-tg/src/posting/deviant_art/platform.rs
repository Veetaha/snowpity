use crate::posting::deviant_art::api::{self, DeviationId};
use crate::posting::deviant_art::{db, Config};
use crate::posting::platform::prelude::*;
use crate::prelude::*;
use crate::Result;
use async_trait::async_trait;
use std::collections::BTreeSet;

pub(crate) struct Platform {
    // client: api::Client,
    // db: db::BlobCacheRepo,
}

impl PlatformTypes for Platform {
    type PostId = DeviationId;
    type BlobId = ();
    type RequestId = DeviationId;
    // type DistinctPostMeta = DistinctPostMeta;
}

#[async_trait]
impl PlatformTrait for Platform {
    type Config = Config;

    const NAME: &'static str = "DeviantArt";

    fn new(params: PlatformParams<Config>) -> Self {
        Self {
            client: api::Client::new(params.config, params.http),
            db: db::BlobCacheRepo::new(params.db),
        }
    }

    fn parse_query(query: &str) -> ParseQueryResult<'_, DeviationId> {
        if let Some((_, host, author, art, id)) =
            parse_with_regexes!(query, r"((?:www.)?deviantart\.com)/(.+/)?art/(.+)-(\d+)")
        {
            let id = id.parse().ok()?;
            let art = art.to_owned();

            if author.is_empty() {
                return Some(DeviationId::ArtAndId { art, id });
            }

            let author = author.to_owned();

            return Some(DeviationId::Full { author, art, id });
        }

        let (_, host, id) = parse_with_regexes!(
            query,
            r"(deviantart\.com/deviation)/(\d+)",
            r"(view.deviantart\.com)/(\d+)",
        )?;

        Some(DeviationId::Id(id.parse().ok()?))
    }

    async fn get_post(&self, media: DeviationId) -> Result<Post<Self>> {
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

    async fn get_cached_blobs(&self, media: DeviationId) -> Result<Vec<CachedBlobId<Self>>> {
        Ok(Vec::from_iter(
            self.db
                .get(media)
                .with_duration_log("Reading the cache from the database")
                .await?
                .map(CachedBlobId::with_tg_file),
        ))
    }

    async fn set_cached_blob(&self, media: DeviationId, blob: CachedBlobId<Self>) -> Result {
        self.db.set(media, blob.tg_file).await
    }
}

#[derive(Clone)]
pub(crate) struct DistinctPostMeta {
    is_adult: bool,
}

impl DistinctPostMetaTrait for DistinctPostMeta {
    fn nsfw_ratings(&self) -> Vec<&str> {
        todo!("Standardize the way to handle NSFW ratings")
        // if self.is_adult {
        //     // vec!["nsfw"]
        // } else {
        //     Vec::new()
        // }
    }
}

impl DisplayInFileName for api::DeviationId {
    fn display_in_file_name(&self) -> Option<String> {
        Some(self.numeric().to_string())
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    #[test]
    fn smoke() {
        use crate::posting::platform::tests::assert_parse_query as test;
        test(
            "https://deviantart.com/miltvain/art/Twilight-magic-418078970",
            expect![],
        );
        test(
            "https://www.deviantart.com/miltvain/art/Twilight-magic-418078970",
            expect![],
        );
        test(
            "https://miltvain.deviantart.com/art/Twilight-magic-418078970",
            expect![],
        );
        test(
            "https://deviantart.com/art/Twilight-magic-418078970",
            expect![],
        );
        test(
            "https://www.deviantart.com/art/Twilight-magic-418078970",
            expect![],
        );
        test("https://deviantart.com/deviation/418078970", expect![]);
        test("https://wwww.deviantart.com/deviation/947204791", expect![]);
        test("https://view.deviantart.com/418078970", expect![]);
    }
}
