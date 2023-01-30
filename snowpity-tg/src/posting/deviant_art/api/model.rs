use crate::http;
use reqwest::Url;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr, PickFirst};

http::def_url_base!(www_deviantart_com, "https://www.deviantart.com");

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
    version: String,

    #[serde(rename = "type")]
    kind: OembedResourceType,

    url: Url,

    author_name: String,

    author_url: Url,

    safety: Option<Safety>,

    #[serde_as(as = "PickFirst<(_, DisplayFromStr)>")]
    width: u64,

    height: u64,

    imagetype: ImageType,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OembedResourceType {
    Photo,
}

#[derive(strum::EnumString, serde_with::DeserializeFromStr, Clone)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum Safety {
    Nonadult,
    Adult,

    #[strum(default)]
    Unknown(String),
}

#[derive(strum::EnumString, serde_with::DeserializeFromStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum ImageType {
    Png,

    #[strum(default)]
    Unknown(String),
}

/// Union of all possible information that can be parsed from a DeviantArt deviation URL.
///
/// DeviantArt doesn't seem to have documentation on URLs, so the following may break.
///
/// The infromation was gathered from various unofficial resources and experimentation.
/// Used resources:
/// - [Syfaro/foxbot source code](https://github.com/Syfaro/foxbot/blob/e1d0c97c77014c4bedb91577407e067e71a9a504/src/sites/mod.rs#L1611)
/// - [Permalinks article](https://www.deviantart.com/ginkgowerkstatt/journal/Did-You-Know-Permalinks-456038680)
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
    ///   Only `www` subdomain is suppported by oembed API. Links without of
    ///   this format, but without the `www` domain perfix will be rejected with 404
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
