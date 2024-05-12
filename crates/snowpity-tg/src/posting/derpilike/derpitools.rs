use crate::posting::derpilike::api::{self, MediaId};
use crate::posting::derpilike::db;
use crate::posting::derpilike::*;

pub(crate) struct Derpitools {
    api: api::Client,
    db: db::BlobCacheRepo,
    platform: DerpiPlatformKind,
}

impl Derpitools {
    pub(crate) fn new(params: PlatformParams<Config>, platform: DerpiPlatformKind) -> Self {
        Derpitools {
            api: api::Client::new(params.config, params.http, platform),
            db: db::BlobCacheRepo::new(params.db, platform.db_table_name()),
            platform,
        }
    }

    pub(crate) async fn get_post<Platform: PlatformTrait<BlobId = (), PostId = MediaId>>(
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
        let repr = best_tg_reprs(&media, self.platform).map_collect(|(download_url, kind)| {
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

    pub(crate) async fn get_cached_blobs<Platform: PlatformTrait<BlobId = ()>>(
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

    pub(crate) async fn set_cached_blob<Platform: PlatformTrait<BlobId = ()>>(
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
    Manebooru,
    Ponerpics,
    Ponybooru,
    Twibooru,
    Furbooru,
}

impl DerpiPlatformKind {
    pub(crate) fn content_kind(self) -> &'static str {
        match self {
            DerpiPlatformKind::Twibooru => "posts",
            _ => "images",
        }
    }

    pub(crate) fn db_table_name(self) -> &'static str {
        match self {
            DerpiPlatformKind::Derpibooru => "derpibooru",
            DerpiPlatformKind::Furbooru => "furbooru",
            DerpiPlatformKind::Manebooru => "manebooru",
            DerpiPlatformKind::Ponerpics => "ponerpics",
            DerpiPlatformKind::Ponybooru => "ponybooru",
            DerpiPlatformKind::Twibooru => "twibooru",
        }
    }

    pub(crate) fn base_url(self) -> Url {
        let url = match self {
            DerpiPlatformKind::Derpibooru => "https://derpibooru.org",
            DerpiPlatformKind::Furbooru => "https://furbooru.org",
            DerpiPlatformKind::Manebooru => "https://manebooru.art",
            DerpiPlatformKind::Ponerpics => "https://ponerpics.org",
            DerpiPlatformKind::Ponybooru => "https://ponybooru.org",
            DerpiPlatformKind::Twibooru => "https://twibooru.org",
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
        // TODO ?
        let base: &[&'static str] = match self {
            DerpiPlatformKind::Twibooru => &["api", "v3"],
            _ => &["api", "v1", "json"],
        };
        let base = base.iter().map(Either::Left);
        let segments = segments.into_iter().map(Either::Right);

        self.url(itertools::chain(base, segments))
    }
}
