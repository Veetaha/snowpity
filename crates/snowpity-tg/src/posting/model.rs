use super::platform::prelude::*;
use super::{AllPlatforms, Mirror};
use crate::prelude::*;
use crate::util::units::MB;
use crate::{tg, Result};
use derivative::Derivative;
use itertools::Itertools;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use reqwest::Url;
use std::collections::BTreeSet;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile, Message};
use teloxide::utils::markdown;

pub(crate) struct UploadMethodSizeRestriction {
    /// Maximum size of the blob that is possible to upload via passing a URL to Telegram.
    /// It is the most efficient way to upload a blob, because Telegram will download it
    /// without any additional traffic from our side.
    pub(crate) by_url: u64,

    /// Maximum size of a photo that is possible to upload via multi-part request to Telegram.
    /// It is the least efficient way to upload a blob, because we need to do an intermediate
    /// download of the blob and then upload it to Telegram.
    pub(crate) by_multipart: u64,
}

/// Telegram size restrictions for photo uploads
pub(crate) const MAX_TG_PHOTO_SIZE: UploadMethodSizeRestriction = UploadMethodSizeRestriction {
    by_url: 5 * MB,
    by_multipart: 10 * MB,
};

/// Telegram size restrictions for document and video uploads
pub(crate) const MAX_TG_FILE_SIZE: UploadMethodSizeRestriction = UploadMethodSizeRestriction {
    by_url: 20 * MB,
    by_multipart: 50 * MB,
};

#[derive(Debug, Clone, Copy, strum::IntoStaticStr, strum::Display)]
pub(crate) enum BlobKind {
    ImageJpeg,
    ImagePng,
    ImageSvg,
    VideoMp4,

    /// Soundless MP4 video is considered to be an animation
    AnimationMp4,

    // TODO(Havoc)
    /// Use this only if MP4 is not supported from the source.
    /// Webm file will be converted to MP4 via ffmpeg.
    VideoWebm,

    /// Best not to have gifs, but MP4s. Use this only if MP4 is not supported
    /// from the source. It means we'll need to convert the gif to MP4.
    AnimationGif,
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
    pub(crate) blob: UniBlob<Service>,
    pub(crate) tg_file: TgFileMeta,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MediaDimensions {
    pub(crate) width: u64,
    pub(crate) height: u64,
}

/// The sizes are measured in bytes
#[derive(Clone, Debug)]
pub(crate) enum BlobSizeHint {
    /// The upper bound of the file size is known.
    /// For example, such information can be obtained from the media hosting
    /// platform docs, where they set limits on file sizes.
    ///
    /// It's better to have a known upper bound info than nothing at all,
    /// if exact size isn't specified in the API response.
    Max(u64),

    Unknown,
}

#[derive(Clone)]
pub(crate) enum SafetyRating {
    /// SFW - safe for work
    Sfw,

    /// NSFW - not safe for work
    Nsfw {
        /// Describes the the kind of NSFW that may be seen in the content
        /// with more detail.
        ///
        /// Not all platforms provide more level of detail
        /// than just SFW/NSFW, so this field may be empty.
        ///
        /// Examples: "suggestive" or "questionable" (Derpibooru).
        kinds: Vec<String>,
    },
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
    pub(crate) safety: SafetyRating,
}

/// Metadata about the post that includes the [`BasePostMeta`] and
/// the list of blobs attached to the post without caching information.
pub(crate) struct Post<Service: PlatformTypes = AllPlatforms> {
    pub(crate) base: BasePost<Service>,

    /// List of blobs attached to the post. It may be empty
    pub(crate) blobs: Vec<MultiBlob<Service>>,
}

/// Metadata about the post that is essentially an extension of [`Post`],
/// but adds caching information to each blob.
#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub(crate) struct CachedPost<Service: PlatformTypes = AllPlatforms> {
    pub(crate) base: BasePost<Service>,

    /// If present, denotes that this post was requested from the mirror of
    /// the original posting platform.
    pub(crate) mirror: Option<Service::Mirror>,

    /// List of blobs attached to the post. It may be empty
    pub(crate) blobs: Vec<CachedBlobId<Service>>,
}

#[derive(Debug, Clone)]
pub(crate) struct BlobRepr {
    /// Describes whether this is an image, video, or an animation
    pub(crate) kind: BlobKind,

    /// The dimensions of the blob, if it is a visual kind of blob (which it always is today).
    /// May be `None` if the dimensions are unknown.
    pub(crate) dimensions: Option<MediaDimensions>,

    /// Size hint of the blob in bytes if known. It should not be considered
    /// as a reliable source of information. It may be inaccurate.
    pub(crate) size_hint: BlobSizeHint,

    /// URL of the resource where the blob can be downloaded from
    pub(crate) download_url: Url,
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub(crate) struct UniBlob<Platform: PlatformTypes = AllPlatforms> {
    /// Unique identifier of the blob specific to the posting platform
    pub(crate) id: Platform::BlobId,
    pub(crate) repr: BlobRepr,
}

/// Metadata about a blob
#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub(crate) struct MultiBlob<Platform: PlatformTypes = AllPlatforms> {
    /// Unique identifier of the blob specific to the posting platform
    pub(crate) id: Platform::BlobId,

    /// Different formats of the blob sorted from the most preferred to the least preferred.
    ///
    /// This is most important for GIFs in derpibooru. Derpibooru doesn't
    /// have MP4 representation for some GIFs, and in this case we would fallback
    /// to generating our own from the GIF.
    pub(crate) repr: Vec<BlobRepr>,
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
    /// The author used AI to create media
    Prompter,
}

impl BasePost {
    fn prefer_mirror_url(mirror: Option<&Mirror>, url: Url) -> Url {
        mirror.map(|mirror| mirror.mirror_url(url)).unwrap_or(url)
    }

    pub(crate) fn caption(&self, mirror: Option<&Mirror>) -> String {
        // FIXME: ensure the caption doesn't overflow 1024 characters
        let authors: Vec<_> = self.authors.iter().map_collect(|author| {
            let author_entry = match author.kind {
                Some(AuthorKind::Editor) => " (editor)",
                Some(AuthorKind::Prompter) => " (prompter)",
                None => "",
            };
            let author_entry = format!("{}{}", author.name, author_entry);
            let author_url = Self::prefer_mirror_url(mirror, author.web_url.clone());

            markdown::link(author_url.as_str(), &markdown::escape(&author_entry))
        });

        let authors = match authors.as_slice() {
            [] => "".to_owned(),
            _ => format!(" by {}", authors.iter().format(", ")),
        };

        let mut nsfw_ratings = self.safety.nsfw_ratings().peekable();
        let nsfw_ratings = match nsfw_ratings.peek() {
            None => "".to_owned(),
            Some(_) => format!(" ({})", nsfw_ratings.format(", ")),
        };

        let nsfw_ratings = markdown::escape(&nsfw_ratings);

        let source = mirror
            .map(ToString::to_string)
            .unwrap_or_else(|| self.id.platform_name().to_owned());

        let post_url = Self::prefer_mirror_url(mirror, self.web_url);

        format!(
            "*{}{authors}{nsfw_ratings}*",
            markdown::link(
                post_url.as_str(),
                &markdown::escape(&format!("Source ({source})"))
            ),
        )
    }
}

impl<Platform: PlatformTypes<BlobId = ()>> MultiBlob<Platform> {
    pub(crate) fn from_single(repr: BlobRepr) -> Self {
        let repr = vec![repr];
        Self { id: (), repr }
    }
}

impl UniBlob {
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

        let ratings = join(&mut post.safety.nsfw_ratings());
        let authors = join(&mut post.authors.iter().map(|artist| artist.name.as_str()));

        let platform = post.id.platform_name().to_lowercase();

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
        let file_extension = self.repr.kind.processed_file_extension();

        format!("{segments}.{file_extension}")
    }
}

impl SafetyRating {
    /// Simple conditional creation of [`SafetyRating::Sfw`] or [`SafetyRating::nsfw()`].
    pub(crate) fn sfw_if(condition: bool) -> Self {
        if condition {
            Self::Sfw
        } else {
            Self::nsfw()
        }
    }

    /// Returns [`SafetyRating::Nsfw`] with no additional information about the
    /// rating. Use this only if the posting platform's API doesn't provide more
    /// detailed information about the safety rating, than just SFW/NSFW switch
    pub(crate) fn nsfw() -> Self {
        Self::Nsfw { kinds: vec![] }
    }

    fn nsfw_ratings(&self) -> impl Iterator<Item = &str> {
        use itertools::Either::{Left, Right};
        match self {
            Self::Sfw => Left(std::iter::empty()),
            Self::Nsfw { kinds } => Right(
                kinds
                    .iter()
                    .map(String::as_str)
                    .chain(kinds.is_empty().then_some("nsfw")),
            ),
        }
    }
}

impl MediaDimensions {
    pub(crate) fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }
}

impl BlobSizeHint {
    pub(crate) fn max_mb(megabytes: u64) -> Self {
        Self::Max(megabytes * MB)
    }

    /// Maximum size of the file in bytes if known, otherwise zero
    pub(crate) fn to_max_or_zero(&self) -> u64 {
        match self {
            Self::Max(bytes) => *bytes,
            Self::Unknown => 0,
        }
    }
}

impl BlobKind {
    /// Extension of the file after it was processed by us. For example, if
    /// the original [`BlobKind`] was [`BlobKind::AnimationGif`], then we
    /// convert it to soundless MP4, because Telegram's GIF support is bad.
    fn processed_file_extension(&self) -> &'static str {
        match self {
            BlobKind::ImageJpeg => "jpg",
            BlobKind::ImagePng => "png",
            BlobKind::ImageSvg => "svg",
            BlobKind::VideoMp4
            | BlobKind::VideoWebm
            | BlobKind::AnimationMp4
            | BlobKind::AnimationGif => "mp4",
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
    pub(crate) fn into_cached(
        self,
        mirror: Option<Service::Mirror>,
        blobs: Vec<CachedBlobId<Service>>,
    ) -> CachedPost<Service> {
        CachedPost {
            base: self,
            mirror,
            blobs,
        }
    }
}
