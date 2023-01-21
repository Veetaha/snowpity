mod model;
mod services;

pub(crate) mod derpi;
pub(crate) mod twitter;

use crate::{http, Result};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::fmt;
use std::hash::Hash;

pub(crate) use model::*;
pub(crate) use services::*;

// The name of the media host, e.g. "derpibooru.org" and the request ID
type ParseQueryResult<'i, R> = Option<(&'i str, R)>;
type ResolvePostResult = Result<(PostMeta, Vec<CachedBlob>)>;

struct ServiceParams<C> {
    config: C,
    http: http::Client,
    db: sqlx::PgPool,
}

#[async_trait]
trait ServiceTrait {
    const NAME: &'static str;

    type PostId: fmt::Debug + Clone + PartialEq + Eq + Hash;
    type BlobId: fmt::Debug + Clone + PartialEq + Eq + Hash;
    type RequestId: fmt::Debug + Clone + PartialEq + Eq + Hash;
    type Config: DeserializeOwned;
    type DistinctPostMeta: DistinctPostMetaTrait;

    fn new(params: ServiceParams<Self::Config>) -> Self;

    fn parse_query(query: &str) -> ParseQueryResult<'_, Self::RequestId>;

    /// Fetch metadata about the post from the hosting.
    async fn get_post_meta(&self, request: Self::RequestId) -> Result<PostMeta>;

    /// Get the cached version of the blobs from the database
    async fn get_cached_blobs(&self, request: Self::RequestId) -> Result<Vec<CachedBlob>>;

    /// Save the information about the file uploaded to Telegram in the database.
    async fn set_cached_blob(
        &self,
        post: Self::PostId,
        blob: Self::BlobId,
        tg_file: TgFileMeta,
    ) -> Result;

    /// Combines both getting the post meta, and getting the cached blobs.
    ///
    /// Getting the post meta from the posting platform will dominate
    /// the time spent in this function, so reaching out to the
    /// cache almost doesn't influence the latency of the request.
    async fn resolve_post(&self, request: Self::RequestId) -> ResolvePostResult {
        futures::try_join!(self.get_post_meta(request), self.get_cached_blobs(request),)
    }
}

trait DistinctPostMetaTrait: Clone {
    fn nsfw_ratings(&self) -> Vec<&str>;
}

trait ConfigTrait {
    const ENV_PREFIX: &'static str;
}

trait DisplayInFileName {
    /// Displays the ID in the file name. If returns `None`, then the ID
    /// won't be inserted into the file name
    fn display_in_file_name(&self) -> Option<String>;
}

impl DisplayInFileName for () {
    fn display_in_file_name(&self) -> Option<String> {
        None
    }
}

/// Utility macro for request parser implementations
macro_rules! parse_with_regexes {
    ($str:ident, $($regex:literal),* $(,)?) => {
        None$(.or_else(|| ::lazy_regex::regex_captures!($regex, $str)))*
    }
}

use parse_with_regexes;

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{expect, Expect};

    #[track_caller]
    pub(crate) fn assert_parse_query(query: &str, expected: Expect) {
        let actual = if let Some((media_host, id)) = Services::parse_query(query) {
            format!("{media_host}:{id:?}")
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
