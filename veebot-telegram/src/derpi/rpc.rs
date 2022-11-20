//! Declarations of the derpibooru JSON API types.
//! Use TypeScript declarations as a reference (though they may go out of date):
//! https://github.com/octet-stream/dinky/blob/master/lib/Dinky.d.ts
use crate::derpi::derpi;
use chrono::prelude::*;
use derive_more::{Display, FromStr};
use reqwest::Url;
use serde::Deserialize;

#[derive(Display, FromStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct MediaId(u64);

impl crate::util::DbRepresentable for MediaId {
    type DbRepr = i64;
}

impl crate::util::TryIntoDbImp for MediaId {
    fn try_into_db_imp(self) -> Self::DbRepr {
        self.0.try_into_db_imp()
    }
}


#[derive(Debug, Deserialize)]
pub(crate) struct SearchImagesResponse {
    pub(crate) images: Vec<Media>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GetImageResponse {
    pub(crate) image: Media,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Media {
    pub(crate) id: MediaId,
    pub(crate) mime_type: MimeType,
    pub(crate) representations: ImageRepresentations,
    pub(crate) tags: Vec<String>,
    pub(crate) created_at: DateTime<Utc>,
    /// The image's number of upvotes minus the image's number of downvotes.
    pub(crate) score: i64,
    pub(crate) size: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ImageRepresentations {
    pub(crate) full: Url,
    pub(crate) thumb: Url,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) enum MimeType {
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

impl Media {
    pub(crate) fn webpage_url(&self) -> Url {
        media_id_to_webpage_url(self.id)
    }
}

pub(crate) fn media_id_to_webpage_url(media_id: MediaId) -> Url {
    derpi(["images", &media_id.to_string()])
}

impl MimeType {
    pub(crate) fn is_image(&self) -> bool {
        use MimeType::*;
        match self {
            ImageGif | ImageJpeg | ImagePng | ImageSvgXml => true,
            VideoWebm => false,
        }
    }
}
