mod client;
mod db;
mod parsing;

use super::{parse_with_regexes, ParseQueryResult};
use client::*;
use serde::Deserialize;

pub(crate) use client::{TweetId, MediaKey};
pub(crate) use parsing::parse_query;

pub(super) type RequestId = TweetId;
pub(super) type PostId = TweetId;
pub(super) type BlobId = MediaKey;

#[derive(Clone, Deserialize)]
pub(crate) struct Config {
    bearer_token: String,
}

pub(crate) struct DistinctPostMeta {
    /// If true the tweet may contain mature content
    possibly_sensitive: bool,
}

impl DistinctPostMeta {
    pub(crate) fn nsfw_ratings(&self) -> Vec<&str> {
        if self.possibly_sensitive {
            return vec!["nsfw"]
        }
        vec![]
    }
}

pub(crate) struct Service {
    client: Client,
    db: db::MediaCacheRepo,
}

impl super::ServiceTrait for Service {
    type PostId;

    type BlobId;

    type RequestId;

    type Config;

    fn new(params:super::ServiceParams<Self::Config>) -> Self {
        todo!()
    }

    fn parse_query(str: &str) -> ParseQueryResult<'_,Self::RequestId> {
        todo!()
    }

    async fn get_post_meta(&self,request: &Self::RequestId) -> crate::Result<super::model::PostMeta> {
        todo!()
    }
}
