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
    api_key: String,
}

pub(crate) struct Client {
    http: http::Client,
    cfg: Config,
}

impl Client {
    pub(crate) fn new(cfg: Config, http: http::Client) -> Self {
        // Derpibooru API is rate-limited to 3 requests per second as per their response in discord:
        // https://discord.com/channels/430829008402251796/438029140659142657/1048823724364800101
        //
        // The http client should already handle exponential backoff with retries.
        Self { http, cfg }
    }

    pub(crate) async fn get_media(&self, media_id: MediaId) -> Result<Media> {
        Ok(self
            .http
            .get(derpi_api(["images", &media_id.to_string()]))
            .query(&[("key", self.cfg.api_key.as_str())])
            .read_json::<GetImageResponse>()
            .await?
            .image)
    }
}
