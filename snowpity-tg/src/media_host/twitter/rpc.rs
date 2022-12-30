use super::TwitterError;
use crate::{err, Result};
use reqwest::Url;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum ResponseResult<D, I> {
    Ok(Response<D, I>),
    Err(Errors),
}

#[derive(Debug, Deserialize)]
pub(super) struct Response<D, I> {
    pub(super) data: D,
    pub(super) includes: I,
}

#[derive(Debug, Deserialize)]
pub(super) struct Errors {
    pub(super) errors: Vec<Error>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Error {
    pub(crate) title: String,
    pub(crate) detail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GetTweetIncludes {
    pub(super) media: Vec<Media>,
    pub(super) users: Vec<User>,
}

#[serde_as]
#[derive(
    derive_more::Display, derive_more::FromStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(transparent)]
pub(crate) struct TweetId(#[serde_as(as = "DisplayFromStr")] pub(super) u64);

#[derive(derive_more::Display, Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub(crate) struct MediaKey(String);

#[derive(Debug, Deserialize)]
pub(crate) struct Tweet {
    pub(crate) id: TweetId,
    pub(crate) text: String,

    pub(crate) possibly_sensitive: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Media {
    pub(crate) media_key: MediaKey,
    pub(crate) width: u32,
    pub(crate) height: u32,

    #[serde(flatten)]
    pub(crate) kind: MediaKind,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum MediaKind {
    Photo(MediaPhoto),
    AnimatedGif(MediaNonStatic),
    Video(MediaNonStatic),
}

#[derive(Debug, Deserialize)]
pub(crate) struct MediaPhoto {
    pub(crate) url: Url,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MediaNonStatic {
    pub(crate) variants: Vec<MediaVariant>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MediaVariant {
    pub(crate) url: Url,
    pub(crate) content_type: String,
}

#[serde_as]
#[derive(
    derive_more::Display, derive_more::FromStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(transparent)]
pub(crate) struct UserId(#[serde_as(as = "DisplayFromStr")] pub(super) u64);

#[derive(Debug, Deserialize)]
pub(crate) struct User {
    id: UserId,
    name: String,
}

impl<D, I> ResponseResult<D, I> {
    pub(super) fn into_std_result(self) -> Result<Response<D, I>> {
        match self {
            Self::Ok(response) => Ok(response),
            Self::Err(errors) => Err(match <[_; 1]>::try_from(errors.errors) {
                Ok([raw]) => crate::err!(TwitterError::Service { raw }),
                Err(raw_errors) => err!(TwitterError::ServiceMany { raw_errors }),
            }),
        }
    }
}
