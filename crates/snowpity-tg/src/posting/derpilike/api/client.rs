use crate::http;
use crate::posting::derpilike::api::model::*;
use crate::posting::derpilike::{Config, DerpiPlatformKind};
use crate::prelude::*;
use crate::Result;

/*
TODO: support Derpibooru, Ponerpics, Furbooru...
*/
pub(crate) struct Client {
    http: http::Client,
    derpi_platform: DerpiPlatformKind,
}

impl Client {
    pub(crate) fn new(_cfg: Config, http: http::Client, derpi_platform: DerpiPlatformKind) -> Self {
        // Derpibooru API is rate-limited to 3 requests per second as per their response in discord:
        // https://discord.com/channels/430829008402251796/438029140659142657/1048823724364800101
        //
        // The http client should already handle exponential backoff with retries.
        Self {
            http,
            derpi_platform,
        }
    }

    pub(crate) async fn get_media(&self, media_id: MediaId) -> Result<Media> {
        self.http
            .get(
                self.derpi_platform
                    .api_url(["images", &media_id.to_string()]),
            )
            .read_json::<GetImageResponse>()
            .await?
            .image
            .try_into_media(self.derpi_platform)
    }
}
