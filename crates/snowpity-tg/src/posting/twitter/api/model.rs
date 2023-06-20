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

#[derive(Debug)]
pub(crate) struct Tweet {
    pub(crate) id: TweetId,

    /// Display name of the user who posted the tweet
    pub(crate) name: String,

    pub(crate) username: String,

    pub(crate) photos: Vec<twitter_scraper::Media>,

    pub(crate) videos: Vec<twitter_scraper::Media>,

    pub(crate) gifs: Vec<twitter_scraper::Media>,

    pub(crate) sensitive_content: bool,
}

impl Tweet {
    pub(crate) fn from_raw(id: TweetId, tweet: twitter_scraper::Tweet) -> Self {
        let twitter_scraper::Tweet {
            name,
            username,
            photos,
            videos,
            gifs,
            sensitive_content,
        } = tweet;

        Self {
            id,
            name,
            username,
            photos,
            videos,
            gifs,
            sensitive_content,
        }
    }

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
