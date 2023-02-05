use crate::posting::deviant_art::api::{self, DeviationId};
use crate::posting::deviant_art::{db, Config};
use crate::posting::platform::prelude::*;
use crate::prelude::*;
use crate::Result;
use async_trait::async_trait;

pub(crate) struct Platform {
    api: api::Client,
    db: db::BlobCacheRepo,
}

impl PlatformTypes for Platform {
    type PostId = DeviationId;
    type BlobId = ();
    type RequestId = DeviationId;
}

#[async_trait]
impl PlatformTrait for Platform {
    type Config = Config;

    const NAME: &'static str = "DeviantArt";

    fn new(params: PlatformParams<Config>) -> Self {
        Self {
            api: api::Client::new(params.config, params.http),
            db: db::BlobCacheRepo::new(params.db),
        }
    }

    fn parse_query(query: &str) -> ParseQueryResult<'_, DeviationId> {
        if let Some((_, host, author, art, id)) =
            parse_with_regexes!(query, r"((?:www.)?deviantart\.com)/(?:(.+)/)?art/(.+)-(\d+)")
        {
            let id = id.parse().ok()?;
            let art = art.to_owned();

            if author.is_empty() {
                return Some((host, DeviationId::ArtAndId { art, id }));
            }

            let author = author.to_owned();

            return Some((host, DeviationId::Full { author, art, id }));
        }

        let (_, host, id) = parse_with_regexes!(
            query,
            r"(deviantart\.com/deviation)/(\d+)",
            r"(view.deviantart\.com)/(\d+)",
        )?;

        Some((host, DeviationId::Id(id.parse().ok()?)))
    }

    async fn get_post(&self, deviation: DeviationId) -> Result<Post<Self>> {
        let oembed = self
            .api
            .get_oembed(deviation.clone())
            .instrument(info_span!("Fetching media meta from DeviantArt"))
            .await?;

        let author = Author {
            web_url: oembed.author_url,
            kind: None,
            name: oembed.author_name,
        };

        let dimensions = MediaDimensions {
            width: oembed.width,
            height: oembed.height,
        };

        let file_extension = oembed.url.file_extension().ok_or_else(|| {
            crate::fatal!(
                "DeviantArt returned a URL without a file extension: {}",
                oembed.url
            )
        })?;

        let (kind, size) = match file_extension {
            "png" => (BlobKind::ImagePng, BlobSize::approx_max_direct_photo_url()),
            "jpg" => (BlobKind::ImageJpeg, BlobSize::approx_max_direct_photo_url()),
            _ => {
                return Err(crate::fatal!(
                    "Unsupported DeviantArt file extension: `{file_extension}`",
                ))
            }
        };

        let blob = Blob {
            id: (),
            dimensions,
            // TODO: select best URL
            // Example of the image that displays in original size in browser:
            // https://www.deviantart.com/freeedon/art/Cloudsdale-765869019
            // Example of this image that fits into 2560 square:
            //
            // https://images-wixmp-ed30a86b8c4ca887773594c2.wixmp.com/intermediary/f/
            // aa14a22e-70c1-4301-b452-36b07958ef14/dcnz8bf-d2eb40a7-f56d-43c7-b3f1-f14e0f970380.png
            // /v1/fit/w_2560,h_2560,bl,q_100/cloudsdale_by_freeedon_dcnz8bf.jpg
            //
            //
            // See https://gist.github.com/micycle1/735006a338e4bea1a9c06377610886e7
            // for instructions from someone who reverse-engineered this

            download_url: oembed.url,
            kind,
            // Sizes for images are ~good enough, although not always accurate,
            // but we don't know the size of MP4 equivalent for GIF or WEBM,
            // however those will often fit into the limit of uploading via direct URL.
            size,
        };

        let safety = match oembed.safety {
            Some(api::Safety::Nonadult) => SafetyRating::Sfw,
            Some(api::Safety::Adult) => SafetyRating::nsfw(),
            Some(api::Safety::Other(other)) => {
                warn!(rating = %other, "Faced an unknown DeviantArt safety rating");
                SafetyRating::Nsfw { kinds: vec![other] }
            }
            None => SafetyRating::nsfw(),
        };

        Ok(Post {
            base: BasePost {
                web_url: deviation.to_canonical_url(),
                id: deviation,
                authors: <_>::from_iter([author]),
                safety,
            },
            blobs: vec![blob],
        })
    }

    async fn get_cached_blobs(&self, deviation: DeviationId) -> Result<Vec<CachedBlobId<Self>>> {
        Ok(Vec::from_iter(
            self.db
                .get(deviation.numeric())
                .with_duration_log("Reading the cache from the database")
                .await?
                .map(CachedBlobId::with_tg_file),
        ))
    }

    async fn set_cached_blob(&self, deviation: DeviationId, blob: CachedBlobId<Self>) -> Result {
        self.db.set(deviation.numeric(), blob.tg_file).await
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
