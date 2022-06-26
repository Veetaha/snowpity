//! Symbols related to communicating with the Derpibooru API

use crate::util::prelude::*;
use crate::util::{self, ThemeTag};
use itertools::Itertools;
use std::{collections::HashSet, sync::Arc};
use url::Url;

/// Declarations of the derpibooru JSON API types.
/// Use TypeScript declarations as a reference (though they may go out of date):
/// https://github.com/octet-stream/dinky/blob/master/lib/Dinky.d.ts
pub(crate) mod rpc {
    use chrono::Utc;
    use serde::Deserialize;

    pub(crate) mod search {
        use super::*;

        #[derive(Debug, Deserialize)]
        pub(crate) struct Response {
            pub(crate) images: Vec<Image>,
        }
    }

    #[derive(Debug, Deserialize)]
    pub(crate) struct Image {
        pub(crate) id: u128,
        pub(crate) mime_type: ImageMimeType,
        pub(crate) representations: ImageRepresentations,
        pub(crate) tags: Vec<String>,
        pub(crate) created_at: chrono::DateTime<Utc>,
        /// The image's number of upvotes minus the image's number of downvotes.
        pub(crate) score: u64,
    }

    #[derive(Debug, Deserialize)]
    pub(crate) struct ImageRepresentations {
        pub(crate) full: String,
        pub(crate) thumb: String,
    }

    #[derive(Debug, Deserialize)]
    pub(crate) enum ImageMimeType {
        #[serde(rename = "image/gif")]
        ImageGif,
        #[serde(rename = "image/jpeg")]
        ImageJpeg,
        #[serde(rename = "image/png")]
        ImagePng,
        #[serde(rename = "image/svg+xml")]
        ImageSvgXml,
        #[serde(rename = "video/webm")]
        VideoWebm,
    }
}

util::def_url_base!(derpibooru_api, "https://derpibooru.org/api/v1/json");
util::def_url_base!(derpibooru, "https://derpibooru.org");

impl rpc::Image {
    pub(crate) fn webpage_url(&self) -> Url {
        derpibooru(&["images", &self.id.to_string()])
    }
}

impl rpc::ImageMimeType {
    pub(crate) fn is_image(&self) -> bool {
        match self {
            rpc::ImageMimeType::ImageGif
            | rpc::ImageMimeType::ImageJpeg
            | rpc::ImageMimeType::ImagePng
            | rpc::ImageMimeType::ImageSvgXml => true,
            rpc::ImageMimeType::VideoWebm => false,
        }
    }
}

pub(crate) struct DerpibooruService {
    http_client: Arc<reqwest::Client>,
    derpibooru_api_key: String,
    filter_id: String,
    always_on_tags: HashSet<ThemeTag>,
}

impl DerpibooruService {
    pub(crate) fn new(
        derpibooru_api_key: String,
        filter_id: String,
        always_on_tags: HashSet<ThemeTag>,
        http_client: Arc<reqwest::Client>,
    ) -> Self {
        Self {
            http_client,
            derpibooru_api_key,
            filter_id,
            always_on_tags,
        }
    }

    /// Fetches random pony media (image or video) based on the given tags (if there are any).
    pub(crate) async fn fetch_random_media(
        &self,
        tags: impl IntoIterator<Item = ThemeTag>,
    ) -> crate::Result<Option<rpc::Image>> {
        let tags_with_always_on_ones = tags
            .into_iter()
            .chain(self.always_on_tags.iter().cloned())
            .collect::<HashSet<_>>()
            .iter()
            .join(",");

        let mut query = vec![
            ("sf", "random"),
            ("per_page", "1"),
            ("filter_id", &self.filter_id),
            ("key", &self.derpibooru_api_key),
        ];

        if !tags_with_always_on_ones.is_empty() {
            query.push(("q", &tags_with_always_on_ones));
        }

        let res: rpc::search::Response = self
            .http_client
            .get(derpibooru_api(&["search", "images"]))
            .query(&query)
            .read_json()
            .await?;

        Ok(res.images.into_iter().next())
    }
}
