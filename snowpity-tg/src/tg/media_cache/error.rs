use teloxide::types::MediaKind;
use crate::media_host::derpi;

#[derive(Debug, thiserror::Error)]
pub(crate) enum MediaCacheError {
    #[error("Unexpected media kind for mime type {expected:?}: {media:#?}")]
    UnexpectedMediaKind {
        media: Box<MediaKind>,
        expected: derpi::MimeType,
    },

    #[error(
        "The size of the requested file `{}` bytes \
        exceeds the limit of `{}` bytes",
        humansize::format_size(*actual, humansize::BINARY),
        humansize::format_size(*max, humansize::BINARY),
    )]
    FileTooBig { actual: u64, max: u64 },
}
