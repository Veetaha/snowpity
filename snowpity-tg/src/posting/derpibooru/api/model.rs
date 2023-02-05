//! Declarations of the derpibooru JSON API types.
//! Use [TypeScript declarations] as a reference (though they may go out of date):
//!
//! [TypeScript declarations]: https://github.com/octet-stream/dinky/blob/master/lib/Dinky.d.ts
use crate::posting::derpibooru::api::derpibooru;
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
    // pub(crate) size: u64,
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

    /// Returns all authors of the media. This includes the artists and the editors.
    pub(crate) fn authors(&self) -> impl Iterator<Item = Author> + '_ {
        self.tags.iter().filter_map(move |tag| {
            let (prefix, value) = tag.split_once(':')?;
            AuthorKind::iter()
                .find(|kind| <&'static str>::from(kind) == prefix)
                .map(|kind| Author {
                    kind,
                    name: value.to_owned(),
                })
        })
    }

    pub(crate) fn safety_rating_tags(&self) -> impl Iterator<Item = &str> {
        self.tags
            .iter()
            .map(String::as_str)
            .filter(|tag| SAFETY_RATING_TAGS.contains(tag))
    }

    /// URL of the media that best suits Telegram.
    ///
    /// Right now this is just the `view_url`, i.e. the original image representation.
    /// Best would be if derpibooru could generate the representation of an image for
    /// 2560x2560 pixels, but the biggest non-original representation is 1280x1024,
    /// according to philomena's [sources].
    ///
    /// This doesn't however guarantee the images will have top-notch quality (see [wiki]).
    ///
    /// [wiki]: https://github.com/Veetaha/snowpity/wiki/Telegram-images-compression
    /// [sources]: https://github.com/philomena-dev/philomena/blob/743699c6afe38b20b23f866c2c1a590c86d6095e/lib/philomena/images/thumbnailer.ex#L16-L24
    pub(crate) fn best_tg_url(&self) -> Url {
        use MimeType::*;
        match self.mime_type {
            ImageJpeg | ImagePng | ImageSvgXml => self.view_url.clone(),
            ImageGif | VideoWebm => self.unwrap_mp4_url(),
        }
    }
}

#[derive(Debug, Deserialize, strum::IntoStaticStr, strum::Display, strum::EnumIter)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum AuthorKind {
    Artist,
    Editor,
}

pub(crate) struct Author {
    pub(crate) kind: AuthorKind,
    pub(crate) name: String,
}

impl Author {
    pub(crate) fn web_url(&self) -> Url {
        let mut url = derpibooru(["search"]);
        let tag = format!("{}:{}", self.kind, self.name);
        url.query_pairs_mut().append_pair("q", &tag);
        url
    }
}

impl MediaId {
    pub(crate) fn to_webpage_url(self) -> Url {
        derpibooru(["images", &self.to_string()])
    }
}
