use crate::http;
use crate::posting::derpibooru::api::model::*;
use crate::posting::derpibooru::Config;
use crate::prelude::*;
use crate::Result;

crate::url::def!(pub(crate) derpibooru_api, "https://derpibooru.org/api/v1/json");
crate::url::def!(pub(crate) derpibooru, "https://derpibooru.org");

pub(crate) struct Client {
    http: http::Client,
}

impl Client {
    pub(crate) fn new(_cfg: Config, http: http::Client) -> Self {
        // Derpibooru API is rate-limited to 3 requests per second as per their response in discord:
        // https://discord.com/channels/430829008402251796/438029140659142657/1048823724364800101
        //
        // The http client should already handle exponential backoff with retries.
        Self { http }
    }

    pub(crate) async fn get_media(&self, media_id: MediaId) -> Result<Media> {
        Ok(self
            .http
            .get(derpibooru_api(["images", &media_id.to_string()]))
            .read_json::<GetImageResponse>()
            .await?
            .image)
    }
}
