/// The error messages here will be displayed directly to the user in
/// the inline query results, so keep them extermely short!
#[derive(Debug, thiserror::Error)]
pub(crate) enum MediaCacheError {
    #[error("Unexpected media kind for mime type {expected:?}: {actual:#?}")]
    UnexpectedMediaKind {
        actual: Box<teloxide::types::MediaKind>,
        expected: super::MediaKind,
    },

    #[error(
        "Too big ({}, max: {})",
        humansize::format_size(*actual, humansize::BINARY),
        humansize::format_size(*max, humansize::BINARY),
    )]
    FileTooBig { actual: u64, max: u64 },

    #[error(transparent)]
    Twitter(super::twitter::TwitterMediaCacheError)
}
