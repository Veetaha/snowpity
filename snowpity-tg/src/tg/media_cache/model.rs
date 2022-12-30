use crate::media_host::{derpi, twitter};
use crate::{media_host, tg};
use itertools::Itertools;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use reqwest::Url;
use std::collections::BTreeSet;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile, Message};
use teloxide::utils::markdown;

#[derive(Clone, Copy)]
pub(crate) enum MediaKind {
    ImageJpg,
    ImagePng,
    ImageSvg,
    VideoMp4,

    /// Soundless MP4 video is considered to be an animation
    AnimationMp4,
}

/// Determines the API method used when the media was uploaded to Telegram.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    IntoPrimitive,
    TryFromPrimitive,
    strum::Display,
    strum::IntoStaticStr,
    sqlx::Type,
)]
#[repr(i16)]
pub(crate) enum TgFileKind {
    Photo = 0,
    Document = 1,
    Video = 2,
    Mpeg4Gif = 3,
}

sqlx_bat::impl_try_into_from_db_via_std!(TgFileKind, i16);

impl TgFileKind {
    pub(crate) async fn upload(
        self,
        bot: &tg::Bot,
        chat_id: ChatId,
        input_file: InputFile,
        caption: String,
    ) -> Result<Message, teloxide::RequestError> {
        match self {
            Self::Photo => bot.send_photo(chat_id, input_file).caption(caption).await,
            Self::Video => bot.send_video(chat_id, input_file).caption(caption).await,
            Self::Document => {
                bot.send_document(chat_id, input_file)
                    .caption(caption)
                    .await
            }
            Self::Mpeg4Gif => {
                bot.send_animation(chat_id, input_file)
                    .caption(caption)
                    .await
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TgFileMeta {
    pub(crate) id: String,
    pub(crate) kind: TgFileKind,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, from_variants::FromVariants)]
pub(crate) enum MediaId {
    Derpibooru(derpi::MediaId),
    Twitter(twitter::TweetId, twitter::MediaKey),
}

#[derive(Clone)]
pub(crate) struct CachedMedia {
    pub(crate) id: MediaId,
    pub(crate) tg_file: TgFileMeta,
}

#[derive(Clone)]
pub(crate) struct MediaDimensions {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Clone)]
pub(crate) enum FileSize {
    /// The size of the file is known approximately. The margin for error is
    /// low, so we can optimistically consider it as an exact size. We still
    /// fall back to less efficient upload methods if the actual size is
    /// greater than expected.
    Approx(u64),

    /// The upper bound of the file size is known.
    /// For example, such information can be obtained from the media hosting
    /// platform docs, where they set limits on file sizes.
    ///
    /// It's better to have a known upper bound info than nothing at all,
    /// if exact size isn't specified in the API response.
    Max(u64),
}

#[derive(Clone)]
pub(crate) struct MediaMeta {
    /// A set of artists who created the media
    pub(crate) artists: BTreeSet<Artist>,

    /// Link to the web page where the media originates from
    pub(crate) web_url: Url,

    /// URL to the media file where it can be downloaded
    pub(crate) download_url: Url,

    /// Size of the media in bytes if known. It should not be considered
    /// as a reliable source of information. It may be inaccurate.
    pub(crate) size: FileSize,

    /// Unique identifier of the media specific to the hosting platform
    ///
    /// Invariant: the variant of [`MediaId`] must correspond to the variant
    /// of [`MediaHostSpecific`]
    pub(crate) id: MediaId,

    /// Describes wether this is an image, video, or an animation
    pub(crate) kind: MediaKind,

    /// The exact dimensions of the media
    pub(crate) dimensions: MediaDimensions,

    /// Information specific to the media hosting platform
    pub(crate) host_specific: MediaHostSpecific,
}

#[derive(Clone)]
pub(crate) enum MediaHostSpecific {
    Derpibooru {
        /// A set of tags `safe`, `suggestive`, `explicit`, etc.
        ratings: BTreeSet<String>,
    },
    Twitter {
        /// If true the parent tweet may contain mature content
        possibly_sensitive: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Artist {
    /// The main nick name or real name of the artist they is known under
    pub(crate) name: String,

    /// Link to the artist's web page.
    ///
    /// It's either the artist's profile/home page, or a query for their art
    /// if the web site identifies artists by tags (like derpibooru)
    pub(crate) link: Url,
}

impl MediaMeta {
    pub(crate) fn caption(&self) -> String {
        let artists: Vec<_> = self
            .artists
            .iter()
            .map(|artist| {
                markdown::link(
                    artist.link.as_str(),
                    &markdown::escape(artist.name.as_str()),
                )
            })
            .collect();

        let artists = match artists.as_slice() {
            [] => "".to_owned(),
            artists => format!(" by {}", artists.join(", ")),
        };

        let MediaHostSpecific::Derpibooru { ratings } = &self.host_specific;

        let ratings = ratings.iter().join(", ");
        let ratings = if matches!(ratings.as_str(), "" | "safe") {
            "".to_owned()
        } else {
            format!(" \\({}\\)", markdown::escape(&ratings))
        };

        format!(
            "*{}{artists}{ratings}*",
            markdown::link(
                self.web_url.as_str(),
                &markdown::escape(&format!("Source ({})", self.host_specific.hosting_name()))
            ),
        )
    }
}

impl MediaHostSpecific {
    /// Name of the media service that hosts the art.
    fn hosting_name(&self) -> &'static str {
        match self {
            Self::Derpibooru { .. } => "Derpibooru",
            Self::Twitter { .. } => "Twitter",
        }
    }
}

impl MediaDimensions {
    pub(crate) fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }
}

impl FileSize {
    /// Approximate maximum size of the file in bytes
    pub(crate) fn approx_max(&self) -> u64 {
        match self {
            Self::Approx(size) | Self::Max(size) => *size,
        }
    }
}
