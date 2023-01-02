//! Declarations of the derpibooru JSON API types.
//! Use TypeScript declarations as a reference (though they may go out of date):
//! https://github.com/octet-stream/dinky/blob/master/lib/Dinky.d.ts
use super::derpi;
use reqwest::Url;
use serde::Deserialize;

const RATING_TAGS: &[&str] = &[
    "safe",
    "suggestive",
    "questionable",
    "explicit",
    "semi-grimdark",
    "grimdark",
    "grotesque",
];

#[derive(
    derive_more::Display, derive_more::FromStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(transparent)]
pub struct MediaId(u64);

sqlx_bat::impl_try_into_db_via_newtype!(MediaId(u64));

#[derive(Debug, Deserialize)]
pub(crate) struct GetImageResponse {
    pub(crate) image: Media,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Media {
    pub(crate) id: MediaId,
    pub(crate) mime_type: MimeType,
    pub(crate) tags: Vec<String>,

    // pub(crate) created_at: DateTime<Utc>,
    // The number of upvotes minus the number of downvotes.
    // pub(crate) score: i64,
    pub(crate) size: u64,
    pub(crate) view_url: Url,

    // Dimensions of the media
    pub(crate) width: u64,
    pub(crate) height: u64,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
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
    /// Makes sense only for gifs and webms
    pub(crate) fn unwrap_mp4_url(&self) -> Url {
        let mut url = self.view_url.clone();
        let path = url.path();

        let path = path
            .strip_suffix(".gif")
            .or_else(|| path.strip_suffix(".webm"))
            .unwrap_or_else(|| panic!("BUG: tried to use mp4 URL for non-gif or non-webm media"));

        url.set_path(&format!("{path}.mp4"));
        url
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

impl MediaId {
    pub(crate) fn to_webpage_url(self) -> Url {
        derpi(["images", &self.to_string()])
    }
}