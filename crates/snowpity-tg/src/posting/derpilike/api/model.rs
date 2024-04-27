//! Declarations of the derpibooru JSON API types.
//! Use [TypeScript declarations] as a reference (though they may go out of date):
//!
//! [TypeScript declarations]: https://github.com/octet-stream/dinky/blob/master/lib/Dinky.d.ts
use crate::posting::derpilike::DerpiPlatformKind;
use crate::prelude::*;
use crate::Result;
use reqwest::Url;
use serde::Deserialize;
use strum::IntoEnumIterator;

const SAFETY_RATING_TAGS: &[&str] = &[
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
pub(crate) struct MediaId(u64);

sqlx_bat::impl_try_into_db_via_newtype!(MediaId(u64));

#[derive(Debug, Deserialize)]
pub(crate) struct GetImageResponse {
    #[serde(alias = "post")]
    pub(crate) image: RawMedia,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawMedia {
    id: MediaId,
    mime_type: MimeType,
    tags: Vec<String>,

    // pub(crate) created_at: DateTime<Utc>,
    // The number of upvotes minus the number of downvotes.
    // pub(crate) score: i64,
    // pub(crate) size: u64,
    view_url: MaybeRelativeUrl,

    // Dimensions of the media
    width: u64,
    height: u64,
}

impl RawMedia {
    pub(crate) fn try_into_media(self, platform: DerpiPlatformKind) -> Result<Media> {
        let view_url = match self.view_url {
            MaybeRelativeUrl::Absolute(url) => url,
            MaybeRelativeUrl::Relative(relative) => platform
                .base_url()
                .join(&relative)
                .fatal_ctx(|| format!("Invalid URL returned from {platform:?}: '{relative}'"))?,
        };

        Ok(Media {
            id: self.id,
            mime_type: self.mime_type,
            tags: self.tags,
            view_url,
            width: self.width,
            height: self.height,
            platform,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Media {
    pub(crate) id: MediaId,
    pub(crate) mime_type: MimeType,
    pub(crate) tags: Vec<String>,
    pub(crate) view_url: Url,

    // Dimensions of the media
    pub(crate) width: u64,
    pub(crate) height: u64,

    pub(crate) platform: DerpiPlatformKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum MaybeRelativeUrl {
    Absolute(Url),
    Relative(String),
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

    #[serde(rename = "video/mp4")]
    VideoMp4,
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

    /// Returns all authors of the media. This includes the artists and the editors.
    pub(crate) fn authors(&self) -> impl Iterator<Item = Author> + '_ {
        self.tags.iter().filter_map(move |tag| {
            let (prefix, value) = tag.split_once(':')?;
            AuthorKind::iter()
                .find(|kind| <&'static str>::from(kind) == prefix)
                .map(|kind| Author {
                    kind,
                    name: value.to_owned(),
                    platform: self.platform,
                })
        })
    }

    pub(crate) fn safety_rating_tags(&self) -> impl Iterator<Item = &str> {
        self.tags
            .iter()
            .map(String::as_str)
            .filter(|tag| SAFETY_RATING_TAGS.contains(tag))
    }
}

#[derive(Debug, Deserialize, strum::IntoStaticStr, strum::Display, strum::EnumIter)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum AuthorKind {
    Artist,
    Editor,
    Prompter,
}

pub(crate) struct Author {
    pub(crate) kind: AuthorKind,
    pub(crate) name: String,
    platform: DerpiPlatformKind,
}

impl Author {
    pub(crate) fn web_url(&self) -> Url {
        let mut url = self.platform.url(["search"]);
        let tag = format!("{}:{}", self.kind, self.name);
        url.query_pairs_mut().append_pair("q", &tag);
        url
    }
}

impl MediaId {
    pub(crate) fn to_webpage_url(self, derpi_platform: DerpiPlatformKind) -> Url {
        derpi_platform.url([derpi_platform.content_kind(), &self.to_string()])
    }
}
