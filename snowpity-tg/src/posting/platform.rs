use crate::{http, Result};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::fmt;
use std::hash::Hash;

use super::model::*;

pub(crate) mod prelude {
    pub(crate) use crate::posting::model::*;
    pub(crate) use super::{
        parse_with_regexes, DisplayInFileName, DisplayInFileNameViaToString, DistinctPostMetaTrait,
        ParseQueryResult, PlatformParams, PlatformTrait, PlatformTypes, ConfigTrait
    };
}

// The name of the media host, e.g. "derpibooru.org" and the request ID
pub(crate) type ParseQueryResult<'i, R> = Option<(&'i str, R)>;

pub(crate) struct PlatformParams<C> {
    pub(crate) config: C,
    pub(crate) http: http::Client,
    pub(crate) db: sqlx::PgPool,
}

pub(crate) trait PlatformTypes {
    type PostId: fmt::Debug + Clone + PartialEq + Eq + Hash + DisplayInFileName;
    type BlobId: fmt::Debug + Clone + PartialEq + Eq + Hash + DisplayInFileName;
    type RequestId: fmt::Debug + Clone + PartialEq + Eq + Hash;
    type DistinctPostMeta: DistinctPostMetaTrait;
}

#[async_trait]
pub(crate) trait PlatformTrait: Sized + PlatformTypes {
    type Config: DeserializeOwned;

    const NAME: &'static str;

    fn new(params: PlatformParams<Self::Config>) -> Self;

    fn parse_query(query: &str) -> ParseQueryResult<'_, Self::RequestId>;

    /// Fetch metadata about the post from the posting platform.
    async fn get_post(&self, request: Self::RequestId) -> Result<Post<Self>>;

    /// Get the cached version of the blobs from the database
    async fn get_cached_blobs(&self, request: Self::RequestId) -> Result<Vec<CachedBlobId<Self>>>;

    /// Save the information about the file uploaded to Telegram in the database.
    async fn set_cached_blob(&self, post: Self::PostId, blob: CachedBlobId<Self>) -> Result;
}

pub(crate) trait DistinctPostMetaTrait: Clone {
    fn nsfw_ratings(&self) -> Vec<&str>;
}

pub(crate) trait ConfigTrait {
    const ENV_PREFIX: &'static str;
}

pub(crate) trait DisplayInFileName {
    /// Displays the ID in the file name. If returns `None`, then the ID
    /// won't be inserted into the file name
    fn display_in_file_name(&self) -> Option<String>;
}

impl DisplayInFileName for () {
    fn display_in_file_name(&self) -> Option<String> {
        None
    }
}

/// Provides an impl of [`DisplayInFileName`] for types that implement [`ToString`]
pub(crate) trait DisplayInFileNameViaToString: ToString {}

impl<T: DisplayInFileNameViaToString> DisplayInFileName for T {
    fn display_in_file_name(&self) -> Option<String> {
        Some(self.to_string())
    }
}

/// Utility macro for request parser implementations
macro_rules! parse_with_regexes {
    ($str:ident, $($regex:literal),* $(,)?) => {
        None$(.or_else(|| ::lazy_regex::regex_captures!($regex, $str)))*
    }
}

pub(crate) use parse_with_regexes;

#[cfg(test)]
pub(crate) mod tests {
    use crate::posting::all_platforms;
    use expect_test::{expect, Expect};

    #[track_caller]
    pub(crate) fn assert_parse_query(query: &str, expected: Expect) {
        let actual = if let Some((platform, id)) = all_platforms::parse_query(query) {
            format!("{platform}:{id:?}")
        } else {
            "None".to_owned()
        };
        expected.assert_eq(&actual);
    }

    #[test]
    fn query_parsing_fail() {
        use assert_parse_query as test;

        test("123", expect!["None"]);
        test("furbooru.org/images/123/", expect!["None"]);
    }
}
