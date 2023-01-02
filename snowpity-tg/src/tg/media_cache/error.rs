use crate::display::human_size;

/// The error messages here will be displayed directly to the user in
/// the inline query results, so keep them extremely short!
#[derive(Debug, thiserror::Error)]
pub(crate) enum MediaCacheError {
    #[error("Unexpected media kind for mime type {expected:?}: {actual:#?}")]
    UnexpectedMediaKind {
        actual: Box<teloxide::types::MediaKind>,
        expected: super::MediaKind,
    },

    #[error("Too big ({}, max: {})", human_size(*actual), human_size(*max))]
    FileTooBig { actual: u64, max: u64 },

    #[error(transparent)]
    Twitter(super::twitter::TwitterMediaCacheError),
}
