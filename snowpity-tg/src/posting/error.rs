use crate::display::human_size;
use crate::posting::BlobKind;

/// The error messages here will be displayed directly to the user in
/// the inline query results, so keep them extremely short!
#[derive(Debug, thiserror::Error)]
pub(crate) enum PostingError {
    #[error("Unexpected media kind for mime type {expected:?}: {actual:#?}")]
    UnexpectedMediaKind {
        actual: Box<teloxide::types::MediaKind>,
        expected: BlobKind,
    },

    #[error("Too big ({}, max: {})", human_size(*actual), human_size(*max))]
    BlobTooBig { actual: u64, max: u64 },
}
