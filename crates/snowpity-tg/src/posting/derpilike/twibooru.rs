use self::derpitools::Derpitools;
use crate::posting::derpilike::api::MediaId;
use crate::posting::derpilike::*;
use crate::Result;
use async_trait::async_trait;

pub(crate) struct Platform {
    tools: Derpitools,
}

impl PlatformTypes for Platform {
    type PostId = MediaId;
    type BlobId = ();
    type Request = MediaId;
}

#[async_trait]
impl PlatformTrait for Platform {
    type Config = Config;

    const NAME: &'static str = "Twibooru";

    fn new(params: PlatformParams<Config>) -> Self {
        Self {
            tools: Derpitools::new(params, DerpiPlatformKind::Twibooru),
        }
    }

    fn parse_query(query: &str) -> ParseQueryResult<MediaId> {
        let (_, host, id) = parse_with_regexes!(
            query,
            r"(twibooru.org(?:/posts)?)/(\d+)",
            r"(cdn.twibooru.org/img)/\d+/\d+/\d+/(\d+)",
            r"(cdn.twibooru.org/img/(?:view|download))/\d+/\d+/\d+/(\d+)",
        )?;
        Some((host.into(), id.parse().ok()?))
    }

    async fn get_post(&self, media: MediaId) -> Result<Post<Self>> {
        self.tools.get_post(media).await
    }

    async fn get_cached_blobs(&self, media: MediaId) -> Result<Vec<CachedBlobId<Self>>> {
        self.tools.get_cached_blobs(media).await
    }

    async fn set_cached_blob(&self, media: MediaId, blob: CachedBlobId<Self>) -> Result {
        self.tools.set_cached_blob(media, blob).await
    }
}
