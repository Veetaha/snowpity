use crate::media_host::{derpi, twitter};
use crate::tg;
use itertools::Itertools;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use reqwest::Url;
use std::collections::BTreeSet;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile, Message};
use teloxide::utils::markdown;

pub(crate) const KB: u64 = 1024;
pub(crate) const MB: u64 = 1024 * KB;

pub(crate) const MAX_DIRECT_URL_PHOTO_SIZE: u64 = 5 * MB;
pub(crate) const MAX_PHOTO_SIZE: u64 = 10 * MB;

pub(crate) const MAX_DIRECT_URL_FILE_SIZE: u64 = 20 * MB;
pub(crate) const MAX_FILE_SIZE: u64 = 50 * MB;

#[derive(Debug, Clone, Copy, strum::IntoStaticStr, strum::Display)]
pub(crate) enum MediaKind {
    ImageJpeg,
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
    Hash,
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
    #[from_variants(skip)]
    Twitter(twitter::TweetId, twitter::MediaKey),
}

#[derive(Clone)]
pub(crate) struct CachedMedia {
    pub(crate) id: MediaId,
    pub(crate) tg_file: TgFileMeta,
}

#[derive(Clone)]
pub(crate) struct MediaDimensions {
    pub(crate) width: u64,
    pub(crate) height: u64,
}

/// The sizes are measured in bytes
#[derive(Clone, Debug)]
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

    /// The exact maximum size of the file is not know, but it's optimistic
    /// estimate is heuristicially assumed to be this. This value is not a
    /// reliable source of information, but it's better than nothing.
    ApproxMax(u64),
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

    /// Describes whether this is an image, video, or an animation
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
    pub(crate) web_url: Url,
}

impl MediaMeta {
    /// Short name of the file (not more than 255 characters) for the media
    pub(crate) fn tg_file_name(&self) -> String {
        fn sanitize(tag: &str) -> impl std::fmt::Display + '_ {
            tag.chars()
                .flat_map(char::to_lowercase)
                .map(|char| {
                    if char.is_whitespace() {
                        return '-';
                    } else if char.is_alphanumeric() {
                        return char;
                    }
                    '_'
                })
                .format("")
        }

        fn join(parts: &mut dyn Iterator<Item = &str>) -> String {
            let joined = parts.map(sanitize).join("+");
            if joined.chars().count() <= 100 {
                return joined;
            }
            joined.chars().take(97).chain(['.', '.', '.']).collect()
        }

        let ratings = join(&mut self.nsfw_ratings().into_iter());
        let artists = join(&mut self.artists.iter().map(|artist| artist.name.as_str()));

        let media_host = self.host_specific.hosting_name().to_lowercase();

        let prefix = [&media_host, ratings.as_str(), artists.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .format("-");

        // FIXME: this file type detection influences how telegram processes the file.
        // For example, if we send_video with wrong extension, then it will be registered
        // as a document instead of video, even though it will be returned as a video kind in `Message`
        let file_extension = self
            .download_url
            .path()
            .rsplit('.')
            .next()
            .unwrap_or_else(|| self.kind.file_extension());

        format!(
            "{prefix}-{}.{}",
            self.id.display_in_file_name(),
            file_extension
        )
    }

    pub(crate) fn caption(&self) -> String {
        // FIXME: ensure the caption doesn't overflow 1024 characters
        let artists: Vec<_> = self
            .artists
            .iter()
            .map(|artist| {
                markdown::link(
                    artist.web_url.as_str(),
                    &markdown::escape(artist.name.as_str()),
                )
            })
            .collect();

        let artists = match artists.as_slice() {
            [] => "".to_owned(),
            _ => format!(" by {}", artists.iter().format(", ")),
        };

        let nsfw_ratings = self.nsfw_ratings();
        let nsfw_ratings = match nsfw_ratings.as_slice() {
            [] => "".to_owned(),
            _ => format!(" ({})", nsfw_ratings.iter().format(", ")),
        };
        let nsfw_ratings = markdown::escape(&nsfw_ratings);

        format!(
            "*{}{artists}{nsfw_ratings}*",
            markdown::link(
                self.web_url.as_str(),
                &markdown::escape(&format!("Source ({})", self.host_specific.hosting_name()))
            ),
        )
    }

    fn nsfw_ratings(&self) -> Vec<&str> {
        match &self.host_specific {
            MediaHostSpecific::Derpibooru { ratings } => ratings
                .iter()
                .filter(|tag| *tag != "safe")
                .map(String::as_str)
                .collect(),
            MediaHostSpecific::Twitter { possibly_sensitive } => {
                if *possibly_sensitive {
                    vec!["nsfw"]
                } else {
                    vec![]
                }
            }
        }
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
            Self::Approx(bytes) | Self::Max(bytes) | Self::ApproxMax(bytes) => *bytes,
        }
    }
}

impl MediaKind {
    fn file_extension(&self) -> &'static str {
        match self {
            MediaKind::ImageJpeg => "jpg",
            MediaKind::ImagePng => "png",
            MediaKind::ImageSvg => "svg",
            MediaKind::VideoMp4 | MediaKind::AnimationMp4 => "mp4",
        }
    }
}

impl MediaId {
    /// Displays the media ID without the media hosting platform identification
    fn display_in_file_name(&self) -> String {
        match self {
            Self::Derpibooru(media_id) => media_id.to_string(),
            Self::Twitter(tweet_id, media_key) => {
                format!("{tweet_id}-{media_key}")
            }
        }
    }
}
