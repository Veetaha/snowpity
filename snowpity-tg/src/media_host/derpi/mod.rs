use crate::http;
use crate::prelude::*;
use crate::Result;
use serde::Deserialize;

pub(crate) mod rpc;
pub(crate) use rpc::*;

http::def_url_base!(derpi_api, "https://derpibooru.org/api/v1/json");
http::def_url_base!(derpi, "https://derpibooru.org");

#[derive(Clone, Deserialize)]
pub struct Config {
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
            .get(derpi_api(["images", &media_id.to_string()]))
            .read_json::<GetImageResponse>()
            .await?
            .image)
    }
}
