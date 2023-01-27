use super::platform::prelude::*;
use super::AllPlatforms;
use crate::tg;
use derivative::Derivative;
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
pub(crate) enum BlobKind {
    ImageJpeg,
    ImagePng,
    ImageSvg,
    VideoMp4,

    /// Soundless MP4 video is considered to be an animation
    AnimationMp4,
}

/// Determines the API method used when the blob was uploaded to Telegram.
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
        #[rustfmt::skip]
        return match self {
            Self::Photo    => bot.send_photo(chat_id, input_file).caption(caption).await,
            Self::Video    => bot.send_video(chat_id, input_file).caption(caption).await,
            Self::Document => bot.send_document(chat_id, input_file).caption(caption).await,
            Self::Mpeg4Gif => bot.send_animation(chat_id, input_file).caption(caption).await,
        };
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TgFileMeta {
    pub(crate) id: String,
    pub(crate) kind: TgFileKind,
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub(crate) struct CachedBlobId<Service: PlatformTypes = AllPlatforms> {
    pub(crate) id: Service::BlobId,
    pub(crate) tg_file: TgFileMeta,
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub(crate) struct CachedBlob<Service: PlatformTypes = AllPlatforms> {
    pub(crate) blob: Blob<Service>,
    pub(crate) tg_file: TgFileMeta,
}

#[derive(Clone)]
pub(crate) struct MediaDimensions {
    pub(crate) width: u64,
    pub(crate) height: u64,
}

/// The sizes are measured in bytes
#[derive(Clone, Debug)]
pub(crate) enum BlobSize {
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

/// Basic information about the post that doesn't contain the list of blobs
#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub(crate) struct BasePost<Service: PlatformTypes = AllPlatforms> {
    /// Unique identifier of the post specific to the posting platform
    ///
    /// Invariant: the variant of [`PostId`] must correspond to the variant
    /// of [`MediaHostSpecific`]
    pub(crate) id: Service::PostId,

    /// A set of authors who created the media
    pub(crate) authors: BTreeSet<Author>,

    /// Link to the web page where the post originates from
    pub(crate) web_url: Url,

    /// Information specific to the posting platform
    pub(crate) distinct: Service::DistinctPostMeta,
}

/// Metadata about the post that includes the [`BasePostMeta`] and
/// the list of blobs attached to the post without caching information.
pub(crate) struct Post<Service: PlatformTypes = AllPlatforms> {
    pub(crate) base: BasePost<Service>,

    /// List of blobs attached to the post. It may be empty
    pub(crate) blobs: Vec<Blob<Service>>,
}

/// Metadata about the post that is essentially an extension of [`Post`],
/// but adds caching information to each blob.
#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub(crate) struct CachedPost<Service: PlatformTypes = AllPlatforms> {
    pub(crate) base: BasePost<Service>,

    /// List of blobs attached to the post. It may be empty
    pub(crate) blobs: Vec<CachedBlob<Service>>,
}

/// Metadata about a blob
#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub(crate) struct Blob<Service: PlatformTypes = AllPlatforms> {
    /// Unique identifier of the blob specific to the posting platform
    pub(crate) id: Service::BlobId,

    /// Describes whether this is an image, video, or an animation
    pub(crate) kind: BlobKind,

    /// The dimensions of the blob, if it is visual kind of blob (which it always is today)
    pub(crate) dimensions: MediaDimensions,

    /// Size of the blob in bytes if known. It should not be considered
    /// as a reliable source of information. It may be inaccurate.
    pub(crate) size: BlobSize,

    /// URL of the resource where the blob can be downloaded from
    pub(crate) download_url: Url,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Author {
    /// The main nick name or real name of the authors they is known under
    pub(crate) name: String,

    /// The kind of the author that when defined specifies what role the
    /// author played in the creation of the post
    pub(crate) kind: Option<AuthorKind>,

    /// Link to the authors's web page.
    ///
    /// It's either the authors's profile/home page, or a query for their posts
    /// if the web site identifies authors by tags (like derpibooru)
    pub(crate) web_url: Url,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AuthorKind {
    /// The author is not the original creator, but the editor
    Editor,
}

impl BasePost {
    pub(crate) fn caption(&self) -> String {
        // FIXME: ensure the caption doesn't overflow 1024 characters
        let authors: Vec<_> = self
            .authors
            .iter()
            .map(|author| {
                let author_entry = if matches!(author.kind, Some(AuthorKind::Editor)) {
                    format!("{} (editor)", author.name)
                } else {
                    author.name.clone()
                };
                markdown::link(author.web_url.as_str(), &markdown::escape(&author_entry))
            })
            .collect();

        let authors = match authors.as_slice() {
            [] => "".to_owned(),
            _ => format!(" by {}", authors.iter().format(", ")),
        };

        let nsfw_ratings = self.distinct.nsfw_ratings();
        let nsfw_ratings = match nsfw_ratings.as_slice() {
            [] => "".to_owned(),
            _ => format!(" ({})", nsfw_ratings.iter().format(", ")),
        };
        let nsfw_ratings = markdown::escape(&nsfw_ratings);

        format!(
            "*{}{authors}{nsfw_ratings}*",
            markdown::link(
                self.web_url.as_str(),
                &markdown::escape(&format!("Source ({})", self.distinct.platform_name()))
            ),
        )
    }
}

impl Blob {
    /// Short name of the file (not more than 255 characters) for the media
    pub(crate) fn tg_file_name(&self, post: &BasePost) -> String {
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

        let ratings = join(&mut post.distinct.nsfw_ratings().into_iter());
        let authors = join(&mut post.authors.iter().map(|artist| artist.name.as_str()));

        let platform = post.distinct.platform_name().to_lowercase();

        let post_segment = post.id.display_in_file_name();
        let blob_segment = self.id.display_in_file_name();

        let segments = [&platform, ratings.as_str(), authors.as_str()]
            .into_iter()
            .chain(post_segment.as_deref())
            .chain(blob_segment.as_deref())
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

        format!("{segments}.{file_extension}")
    }
}

impl MediaDimensions {
    pub(crate) fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }
}

impl BlobSize {
    pub(crate) fn max_mb(megabytes: u64) -> Self {
        Self::Max(megabytes * MB)
    }

    /// Approximate maximum size of the file in bytes
    pub(crate) fn approx_max(&self) -> u64 {
        match self {
            Self::Approx(bytes) | Self::Max(bytes) | Self::ApproxMax(bytes) => *bytes,
        }
    }

    pub(crate) fn approx_max_direct_photo_url() -> Self {
        Self::ApproxMax(MAX_DIRECT_URL_PHOTO_SIZE)
    }

    pub(crate) fn approx_max_direct_file_url() -> Self {
        Self::Approx(MAX_DIRECT_URL_FILE_SIZE)
    }
}

impl BlobKind {
    fn file_extension(&self) -> &'static str {
        match self {
            BlobKind::ImageJpeg => "jpg",
            BlobKind::ImagePng => "png",
            BlobKind::ImageSvg => "svg",
            BlobKind::VideoMp4 | BlobKind::AnimationMp4 => "mp4",
        }
    }
}

impl<Service: PlatformTypes> CachedBlob<Service> {
    pub(crate) fn to_id(&self) -> CachedBlobId<Service> {
        CachedBlobId {
            id: self.blob.id.clone(),
            tg_file: self.tg_file.clone(),
        }
    }
}

impl<Service> CachedBlobId<Service>
where
    Service: PlatformTypes<BlobId = ()>,
{
    /// Specialized constructor of [`CachedBlob`] that ignores the [`ServiceTypes::BlobId`],
    /// because it is a unit type.
    pub(crate) fn with_tg_file(tg_file: TgFileMeta) -> Self {
        Self { id: (), tg_file }
    }
}

impl<Service: PlatformTypes> BasePost<Service> {
    pub(crate) fn with_cached_blobs(self, blobs: Vec<CachedBlob<Service>>) -> CachedPost<Service> {
        CachedPost { base: self, blobs }
    }
}