//! Declarations of the derpibooru JSON API types.
//! Use TypeScript declarations as a reference (though they may go out of date):
//! https://github.com/octet-stream/dinky/blob/master/lib/Dinky.d.ts
use crate::derpi::derpi;
use derive_more::{Display, FromStr};
use itertools::Itertools;
use reqwest::Url;
use serde::Deserialize;
use std::fmt;

const RATING_TAGS: &[&str] = &[
    "safe",
    "suggestive",
    "questionable",
    "explicit",
    "semi-grimdark",
    "grimdark",
    "grotesque",
];

#[derive(Display, FromStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct MediaId(pub(crate) u64);

#[derive(Debug, Deserialize)]
pub(crate) struct SearchImagesResponse {
    pub(crate) images: Vec<Media>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GetImageResponse {
    pub(crate) image: Media,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Media {
    pub(crate) id: MediaId,
    pub(crate) mime_type: MimeType,
    pub(crate) representations: ImageRepresentations,
    pub(crate) tags: Vec<String>,
    // pub(crate) created_at: DateTime<Utc>,
    // The number of upvotes minus the number of downvotes.
    // pub(crate) score: i64,
    pub(crate) size: u64,
    pub(crate) view_url: Url,

    // Dimensions of the media
    pub(crate) width: u64,
    pub(crate) height: u64,
    pub(crate) aspect_ratio: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ImageRepresentations {
    pub(crate) thumb_small: Url,
}

#[derive(Display, Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MimeType {
    #[serde(rename = "image/gif")]
    #[display(fmt = "image/gif")]
    ImageGif,

    #[serde(rename = "image/jpeg")]
    #[display(fmt = "image/jpeg")]
    ImageJpeg,

    #[serde(rename = "image/png")]
    #[display(fmt = "image/png")]
    ImagePng,

    #[serde(rename = "image/svg+xml")]
    #[display(fmt = "image/svg+xml")]
    ImageSvgXml,

    #[serde(rename = "video/webm")]
    #[display(fmt = "video/webm")]
    VideoWebm,
}

impl Media {
    pub(crate) fn webpage_url(&self) -> Url {
        media_id_to_webpage_url(self.id)
    }

    pub(crate) fn artists(&self) -> impl Iterator<Item = &str> {
        self.tags
            .iter()
            .filter_map(|tag| tag.strip_prefix("artist:"))
    }

    pub(crate) fn rating_tags(&self) -> impl Iterator<Item = &str> {
        self.tags
            .iter()
            .map(String::as_str)
            .filter(|tag| RATING_TAGS.contains(tag))
    }
}

pub(crate) fn artist_to_webpage_url(artist: &str) -> Url {
    let mut url = derpi(["search"]);
    let tag = format!("artist:{artist}");
    url.query_pairs_mut().append_pair("q", &tag);
    url
}

pub(crate) fn media_id_to_webpage_url(media_id: MediaId) -> Url {
    derpi(["images", &media_id.to_string()])
}

pub(crate) fn sanitize_tag(tag: &str) -> impl fmt::Display + '_ {
    tag.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|char| {
            if char.is_whitespace() {
                return Some('-');
            }
            if char.is_alphanumeric() {
                return Some(char);
            }
            Some('_')
        })
        .format("")
}

impl MimeType {
    pub(crate) fn is_image(self) -> bool {
        use MimeType::*;
        match self {
            ImageGif | ImageJpeg | ImagePng | ImageSvgXml => true,
            VideoWebm => false,
        }
    }
    pub(crate) fn file_extension(self) -> &'static str {
        use MimeType::*;
        match self {
            ImageGif => "gif",
            ImageJpeg => "jpg",
            ImagePng => "png",
            ImageSvgXml => "svg",
            VideoWebm => "webm",
        }
    }
}
