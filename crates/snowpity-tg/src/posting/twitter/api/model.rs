use reqwest::Url;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(
    derive_more::Display, derive_more::FromStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(transparent)]
pub(crate) struct TweetId(#[serde_as(as = "DisplayFromStr")] u64);

sqlx_bat::impl_try_into_db_via_newtype!(TweetId(u64));

#[derive(derive_more::Display, Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub(crate) struct MediaKey(String);

sqlx_bat::impl_try_into_db_via_newtype!(MediaKey(String));

/// API docs: <https://github.com/dylanpdx/BetterTwitFix>
#[derive(Debug, Deserialize)]
pub(crate) struct Tweet {
    /// User's handle
    pub(crate) user_name: String,

    /// User's display name that can contain special characters
    pub(crate) user_screen_name: String,

    pub(crate) media_extended: Vec<Media>,

    #[serde(default)]
    pub(crate) possibly_sensitive: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Media {
    pub(crate) id_str: String,

    #[serde(rename = "type")]
    pub(crate) kind: MediaType,

    pub(crate) url: url::Url,

    #[serde(default)]
    pub(crate) size: MediaSize,
}

#[derive(Default, Debug, Deserialize)]
pub(crate) struct MediaSize {
    pub(crate) height: Option<u64>,
    pub(crate) width: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum MediaType {
    Image,
    Gif,
    Video,
}

impl Tweet {
    /// URL to the Twitter web page of the user's profile
    pub(crate) fn author_web_url(&self) -> Url {
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

        let mut web_url = Url::parse("https://x.com").unwrap();

        // To be safe we push the name as a segment instead of interpolating
        // it into the parsed URL string higher to let the url library do
        // the necessary escaping for us in case twitter ever decides to
        // allow special characters in usernames.
        web_url
            .path_segments_mut()
            .unwrap()
            .push(&self.user_screen_name);

        web_url
    }

    /// URL to the user's tweet with the given ID
    pub(crate) fn tweet_url(&self, tweet_id: TweetId) -> Url {
        let mut web_url = self.author_web_url();
        {
            let mut path = web_url.path_segments_mut().unwrap();
            path.extend(["status", &tweet_id.to_string()]);
        }
        web_url
    }
}

impl TweetId {
    #[cfg(test)]
    pub(crate) fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

impl MediaKey {
    pub(crate) fn from_raw(raw: String) -> Self {
        Self(raw)
    }
}
