use crate::fatal;
use crate::http;
use crate::posting::twitter::api::model::*;
use crate::posting::twitter::Config;
use crate::prelude::*;
use crate::Result;

http::def_url_base!(twitter_api, "https://api.twitter.com/2");

pub(crate) struct Client {
    http: http::Client,
    cfg: Config,
}

impl Client {
    pub(crate) fn new(cfg: Config, http: http::Client) -> Self {
        Self { http, cfg }
    }

    pub(crate) async fn get_tweet(&self, id: TweetId) -> Result<GetTweetResponse> {
        let query = [
            ("expansions", "attachments.media_keys,author_id"),
            ("media.fields", "height,url,width,variants"),
            ("tweet.fields", "possibly_sensitive"),
        ];

        let mut response = self
            .http
            .get(twitter_api(["tweets", &id.to_string()]))
            .bearer_auth(&self.cfg.bearer_token)
            .query(&query)
            .read_json::<ResponseResult<Tweet, GetTweetIncludes>>()
            .await?
            .into_std_result()?;

        let author = std::mem::take(&mut response.includes.users)
            .into_iter()
            .next()
            .ok_or_else(|| fatal!("No user in response: {response:#?}"))?;

        Ok(GetTweetResponse {
            author,
            tweet: response.data,
            media: response.includes.media,
        })
    }
}

#[derive(Debug)]
pub(crate) struct GetTweetResponse {
    pub(crate) author: User,
    pub(crate) tweet: Tweet,
    pub(crate) media: Vec<Media>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TwitterError {
    #[error(
        "Error getting tweet. {}: {}",
        raw.title,
        raw.detail.as_deref().unwrap_or("{details are unknown}")
    )]
    Service { raw: Error },

    #[error("Several errors occurred: {raw_errors:#?}")]
    ServiceMany { raw_errors: Vec<Error> },

    #[error("The media is missing MP4 format (media_key: {})", media.media_key)]
    MissingMp4Variant { media: Media },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test(tokio::test)]
    #[ignore]
    async fn manual_sandbox() {
        dotenvy::dotenv().ok();

        let cfg = crate::config::from_env_or_panic("TWITTER_");

        let client = Client::new(cfg, http::create_client());

        let tweet = client
            .get_tweet(TweetId::from_raw(1609634286050623492))
            .await
            .unwrap();

        eprintln!("{tweet:#?}");
    }
}
