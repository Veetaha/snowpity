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
    #[serde(default)]
    pub(super) media: Vec<Media>,
    pub(super) users: Vec<User>,
}

#[serde_as]
#[derive(
    derive_more::Display, derive_more::FromStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(transparent)]
pub(crate) struct TweetId(#[serde_as(as = "DisplayFromStr")] pub(super) u64);

sqlx_bat::impl_try_into_db_via_newtype!(TweetId(u64));

#[derive(derive_more::Display, Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub(crate) struct MediaKey(String);

sqlx_bat::impl_try_into_db_via_newtype!(MediaKey(String));

#[derive(Debug, Deserialize)]
pub(crate) struct Tweet {
    pub(crate) id: TweetId,
    // pub(crate) text: String,
    #[serde(default)]
    pub(crate) possibly_sensitive: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Media {
    pub(crate) media_key: MediaKey,
    pub(crate) width: u64,
    pub(crate) height: u64,

    #[serde(flatten)]
    pub(crate) kind: MediaKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum MediaKind {
    Photo(MediaPhoto),
    AnimatedGif(MediaNonStatic),
    Video(MediaNonStatic),
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MediaPhoto {
    pub(crate) url: Url,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MediaNonStatic {
    pub(crate) variants: Vec<MediaVariant>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MediaVariant {
    pub(crate) url: Url,
    pub(crate) content_type: String,
    pub(crate) bit_rate: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct User {
    pub(crate) name: String,
    pub(crate) username: String,
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

impl User {
    /// URL to the Twitter web page of the user's profile
    pub(crate) fn web_url(&self) -> Url {
        // We could potentially create a link using the user ID, which would be
        // more stable (user IDs are immutable, while usernames can change),
        // but following such a link on mobile phones doesn't work with the
        // Twitter app at the time of this writing.
        //
        // For example a link like this works on desktop, but if we open this
        // on mobile, we'll see Twitter app opening and closing immediately:
        // https://twitter.com/i/user/727893240
        //
        // It's not documented anywhere, but I found this on stackoverflow:
        // https://stackoverflow.com/a/56924385/9259330

        let mut web_url = Url::parse("https://twitter.com").unwrap();

        // To be safe we push the name as a segment instead of interpolating
        // it into the parsed URL string higher to let the url library do
        // the necessary escaping for us in case twitter ever decides to
        // allow special characters in usernames.
        web_url.path_segments_mut().unwrap().push(&self.username);

        web_url
    }

    /// URL to the user's tweet with the given ID
    pub(crate) fn tweet_url(&self, tweet_id: TweetId) -> Url {
        let mut web_url = self.web_url();
        {
            let mut path = web_url.path_segments_mut().unwrap();
            path.extend(["status", &tweet_id.to_string()]);
        }
        web_url
    }
}

impl Media {
    /// URL of the media in the best quality format. At the time of this writing it is:
    ///
    /// - For images, the `orig` format, which is at most 4096x4096 pixels
    /// - For videos and gifs it is the `video/mp4` format with the highest bitrate
    ///
    /// Media URL formatting is described in twitter [API v1.1 docs]
    ///
    /// [API v1.1 docs]: https://developer.twitter.com/en/docs/twitter-api/v1/data-dictionary/object-model/entities#photo_format
    pub(crate) fn best_quality_url(&self) -> Result<Url> {
        let non_static = match &self.kind {
            MediaKind::Photo(photo) => {
                let mut url = photo.url.clone();
                url.query_pairs_mut().append_pair("name", "orig");
                return Ok(url);
            }
            MediaKind::AnimatedGif(non_static) | MediaKind::Video(non_static) => non_static,
        };

        non_static
            .variants
            .iter()
            .filter_map(|variant| {
                let bitrate = variant.bit_rate?;
                (variant.content_type == "video/mp4").then_some((bitrate, &variant.url))
            })
            .max_by_key(|(bitrate, _)| *bitrate)
            .ok_or_else(|| {
                err!(TwitterError::MissingMp4Variant {
                    media: self.clone()
                })
            })
            .map(|(_, url)| url.clone())
    }
}
