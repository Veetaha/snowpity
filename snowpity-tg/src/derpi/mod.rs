use crate::util::prelude::*;
use crate::util::{self, http, ThemeTag};
use crate::Result;
use itertools::Itertools;
use serde::Deserialize;
use std::collections::HashSet;

pub(crate) mod rpc;
pub(crate) use rpc::*;

util::def_url_base!(derpi_api, "https://derpibooru.org/api/v1/json");
util::def_url_base!(derpi, "https://derpibooru.org");

#[derive(Clone, Deserialize)]
pub struct Config {
    api_key: String,

    // Default filter applied to queries
    filter: String,
}

pub(crate) struct DerpiService {
    http_client: http::Client,
    cfg: Config,
}

impl DerpiService {
    pub(crate) fn new(cfg: Config, http_client: http::Client) -> Self {
        // Derpibooru API is rate-limited to 3 requests per second as per their response in discord:
        // https://discord.com/channels/430829008402251796/438029140659142657/1048823724364800101
        //
        // The http client should already handle exponential backoff with retries.
        Self { http_client, cfg }
    }

    pub(crate) async fn get_media(&self, media_id: MediaId) -> Result<Media> {
        Ok(self
            .http_client
            .get(derpi_api(["images", &media_id.to_string()]))
            .read_json::<GetImageResponse>()
            .await?
            .image)
    }

    /// Fetches random pony media (image or video) based on the given tags (if there are any).
    #[allow(unused)]
    pub(crate) async fn get_random_media(
        &self,
        tags: impl IntoIterator<Item = ThemeTag>,
    ) -> Result<Option<Media>> {
        let tags_with_always_on_ones = tags.into_iter().collect::<HashSet<_>>().iter().join(",");

        let mut query = vec![
            ("sf", "random"),
            ("per_page", "1"),
            ("filter_id", &self.cfg.filter),
            ("key", &self.cfg.api_key),
        ];

        if !tags_with_always_on_ones.is_empty() {
            query.push(("q", &tags_with_always_on_ones));
        }

        let res: SearchImagesResponse = self
            .http_client
            .get(derpi_api(["search", "images"]))
            .query(&query)
            .read_json()
            .await?;

        Ok(res.images.into_iter().next())
    }
}
