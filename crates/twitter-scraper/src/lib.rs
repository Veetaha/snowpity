mod error;
mod ffi;

pub use error::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DefaultOnNull};
use url::Url;

/// Must be called once at the start of the main to log in to twitter
pub fn initialize(cookies_json: &str) {
    // SAFETY: there are no additional invariants for this function
    // other than what's defined in its type signature
    let result: Result<(), String> = unsafe {
        ffi::call_raw_json(ffi::bindings::Initialize, "Initialize", cookies_json).unwrap()
    };

    if let Err(err) = result {
        panic!("Failed to log in to twitter: {}", err);
    }
}

pub fn get_tweet(tweet_id: &str) -> Result<Tweet> {
    // SAFETY: there are no additional invariants for this function
    // other than what's defined in its type signature
    let result: Result<Tweet, String> =
        unsafe { ffi::call(ffi::bindings::GetTweet, "GetTweet", tweet_id)? };

    result.map_err(Error::Fatal)
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tweet {
    /// Display name of the user who posted the tweet
    pub name: String,

    pub username: String,

    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub photos: Vec<Media>,

    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub videos: Vec<Media>,

    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub gifs: Vec<Media>,

    /// This doesn't seem to work or maybe it's affected by the user's settings
    pub sensitive_content: bool,
    // We may use the tweet's text in the future
    // pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    #[serde(rename = "ID")]
    pub id: String,

    #[serde(rename = "URL")]
    pub url: Url,
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{expect, Expect};

    #[track_caller]
    fn assert_get_tweet(id: u64, expected: Expect) {
        let tweet = get_tweet(&id.to_string());
        test_bat::json::assert_result_eq(&tweet, &expected);
    }

    #[test]
    #[ignore]
    fn smoke() {
        let _ = dotenvy::dotenv();

        initialize(std::env::var("TWITTER_COOKIES").unwrap().as_str());

        // Single image
        assert_get_tweet(
            1670551964621639681,
            expect![[r#"
                {
                  "name": "suhareo",
                  "username": "3ubcyxarikaaa",
                  "photos": [
                    {
                      "ID": "1670538021265965056",
                      "URL": "https://pbs.twimg.com/media/Fy7xer1XgAAk9NF.jpg"
                    }
                  ],
                  "videos": [],
                  "gifs": [],
                  "sensitive_content": false
                }"#]],
        );

        // Multiple imagges
        assert_get_tweet(
            1670542719461072898,
            expect![[r#"
                {
                  "name": "🎀Lil'Cinnamon/RAFFLE PINNED(SFW only)",
                  "username": "Lil_Cinnamon",
                  "photos": [
                    {
                      "ID": "1670542668349276164",
                      "URL": "https://pbs.twimg.com/media/Fy71tLkXgAQEbZF.jpg"
                    },
                    {
                      "ID": "1670542674921771010",
                      "URL": "https://pbs.twimg.com/media/Fy71tkDX0AIonwY.jpg"
                    },
                    {
                      "ID": "1670542683578695685",
                      "URL": "https://pbs.twimg.com/media/Fy71uETWAAUf2pW.jpg"
                    },
                    {
                      "ID": "1670542713102430210",
                      "URL": "https://pbs.twimg.com/media/Fy71vySWcAI7AY-.jpg"
                    }
                  ],
                  "videos": [],
                  "gifs": [],
                  "sensitive_content": false
                }"#]],
        );

        // Single GIF
        assert_get_tweet(
            1670487415113515008,
            expect![[r#"
                {
                  "name": "♡BlazyPazy♡",
                  "username": "BlazyPazy",
                  "photos": [],
                  "videos": [],
                  "gifs": [
                    {
                      "ID": "1670487405999329281",
                      "URL": "https://video.twimg.com/tweet_video/Fy7DcfRakAExLkl.mp4"
                    }
                  ],
                  "sensitive_content": false
                }"#]],
        );

        // GIF and an image
        assert_get_tweet(
            1580661436132757506,
            expect![[r#"
                {
                  "name": "Twitter",
                  "username": "Twitter",
                  "photos": [
                    {
                      "ID": "1580661428326907904",
                      "URL": "https://pbs.twimg.com/media/Fe-jMcGWQAAFWoG.jpg"
                    }
                  ],
                  "videos": [],
                  "gifs": [
                    {
                      "ID": "1580661428335382531",
                      "URL": "https://video.twimg.com/tweet_video/Fe-jMcIXkAMXK_W.mp4"
                    }
                  ],
                  "sensitive_content": false
                }"#]],
        );

        // Single video
        assert_get_tweet(
            1558884492190035968,
            expect![[r#"
                {
                  "name": "Sethisto",
                  "username": "Sethisto",
                  "photos": [],
                  "videos": [
                    {
                      "ID": "1558883554125176832",
                      "URL": "https://video.twimg.com/ext_tw_video/1558883554125176832/pu/vid/1280x720/1HcmVTNSiXltNttu.mp4?tag=12"
                    }
                  ],
                  "gifs": [],
                  "sensitive_content": false
                }"#]],
        );

        // Non-existing tweet
        assert_get_tweet(0, expect![[r#"Err:Fatal("tweet with ID 0 not found")"#]]);
    }
}
