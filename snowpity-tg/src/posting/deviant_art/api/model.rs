use reqwest::Url;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr, PickFirst};
use crate::util;

util::url::def!(www_deviantart_com, "https://www.deviantart.com");

/// Numeric ID of the deviation. It is not the UUID from the API, but some (probably incremental)
/// number that appears in the suffix of URLs. No documentation exists on this, but let's suppose
/// these are `u64` integers for efficiency.
#[derive(
    derive_more::Display, derive_more::FromStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(transparent)]
pub(crate) struct DeviationNumericId(u64);

sqlx_bat::impl_try_into_db_via_newtype!(DeviationNumericId(u64));

#[serde_as]
#[derive(Debug, Deserialize)]
pub(crate) struct GetOembedResponse {
    // Useful to determine the version of the Oembed response schema in debug representation
    #[allow(dead_code)]
    pub(crate) version: String,

    // Useful for debugging
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub(crate) kind: String,

    pub(crate) url: Url,

    pub(crate) author_name: String,

    pub(crate) author_url: Url,

    pub(crate) safety: Option<Safety>,

    /// XXX: This is very weird, and it is not documented anywhere, but
    /// sometimes this field may be returned as a string, and sometimes as a number.
    ///
    /// Here are examples of both cases:
    /// - Number: https://backend.deviantart.com/oembed?url=https://www.deviantart.com/deviation/754296933
    /// - String: https://backend.deviantart.com/oembed?url=https://www.deviantart.com/deviation/699813776
    ///
    #[serde_as(as = "PickFirst<(_, DisplayFromStr)>")]
    pub(crate) width: u64,

    /// XXX: Same weirdness may potentially be possible as with `width`
    #[serde_as(as = "PickFirst<(_, DisplayFromStr)>")]
    pub(crate) height: u64,
    // There is a field called `imagetype` here, but sometimess it is an empty
    // string like here:
    // - imagetype is the empty string for jpg:
    //   https://backend.deviantart.com/oembed?url=https://www.deviantart.com/deviation/754296933
    // - imagetype is the empty string for gif:
    //   https://backend.deviantart.com/oembed?url=https://www.deviantart.com/deviation/776090835
    // - imagetype is "png":
    //   https://backend.deviantart.com/oembed?url=https://www.deviantart.com/deviation/699813776
}

#[derive(strum::EnumString, serde_with::DeserializeFromStr, Debug)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum Safety {
    Nonadult,
    Adult,

    #[strum(default)]
    Other(String),
}

/// Union of all possible information that can be parsed from a DeviantArt deviation URL.
///
/// DeviantArt doesn't seem to have documentation on URLs, so the following may break.
///
/// The information was gathered from various unofficial resources and experimentation.
/// Used resources:
/// - [Syfaro/foxbot source code](https://github.com/Syfaro/foxbot/blob/e1d0c97c77014c4bedb91577407e067e71a9a504/src/sites/mod.rs#L1611)
/// - [Permalinks article](https://www.deviantart.com/ginkgowerkstatt/journal/Did-You-Know-Permalinks-456038680)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum DeviationId {
    /// Represents several URL formats.
    ///
    /// - The canonical deviation URL with artist name and art title (obvious from website URLs)
    ///   - `https://deviantart.com/miltvain/art/Twilight-magic-418078970`
    ///   - `https://www.deviantart.com/miltvain/art/Twilight-magic-418078970`
    ///
    /// - Old deviation URL format with the artist name in the domain name (from permalinks article)
    ///   - `https://miltvain.deviantart.com/art/Twilight-magic-418078970`
    Full {
        /// Name of the author (or `deviant` in DeviantArt slang)
        author: String,

        /// Sanitized art title.
        ///
        /// It is not the original title of the art, but the one that was processed by DeviantArt
        /// to be valid for being used in URLs.
        art: String,

        id: DeviationNumericId,
    },

    /// Abbreviated deviation URL without the artist name (from Syfaro/foxbot)
    /// - `https://deviantart.com/art/Twilight-magic-418078970`
    /// - `https://www.deviantart.com/art/Twilight-magic-418078970`
    ArtAndId {
        /// See the docs for this field on [`Self::Full`]
        art: String,

        /// See the docs for this field on [`Self::Full`]
        id: DeviationNumericId,
    },

    /// - Deviation URL with numeric deviation ID (from permalinks article).
    ///   Only `www` subdomain is supported by oembed API. Links without of
    ///   this format, but without the `www` domain prefix will be rejected with 404
    ///   (found experimentally as of 2023-01-30).
    ///   - `https://deviantart.com/deviation/418078970`
    ///   - `https://wwww.deviantart.com/deviation/947204791`
    ///
    /// - Deviation URL with the numeric deviation ID on `view` subdomain
    ///   (from permalinks article webpage). Doesn't work with oembed API at all.
    ///   - `https://view.deviantart.com/418078970`
    Id(
        /// See the docs for this field on [`Self::Full`]
        DeviationNumericId,
    ),
}

impl DeviationId {
    /// Returns the best canonical representation of the deviation URL.
    /// The returned URL format should be supported by the DeviantArt oEmbed API.
    pub(crate) fn to_canonical_url(&self) -> Url {
        match self {
            Self::Full { author, art, id } => {
                www_deviantart_com([author, "art", &format!("{art}-{id}")])
            }
            Self::ArtAndId { art, id } => www_deviantart_com(["art", &format!("{art}-{id}")]),
            Self::Id(id) => www_deviantart_com(["deviation", &id.to_string()]),
        }
    }

    pub(crate) fn numeric(&self) -> DeviationNumericId {
        match self {
            Self::Full { id, .. } => *id,
            Self::ArtAndId { id, .. } => *id,
            Self::Id(id) => *id,
        }
    }
}
