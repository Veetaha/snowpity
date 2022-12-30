use crate::prelude::*;
use crate::{err, fatal, http, Result};
use serde::Deserialize;

pub(crate) mod rpc;
pub(crate) use rpc::*;

http::def_url_base!(twitter_api, "https://api.twitter.com/2");
// http::def_url_base!(derpi, "https://derpibooru.org");

#[derive(Clone, Deserialize)]
pub struct Config {
    bearer_token: String,
}

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
            ("media.fields", "height,type,url,width,variants"),
            ("tweet.fields", "id,text,possibly_sensitive"),
            ("user.fields", "id,name"),
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
    author: User,
    tweet: Tweet,
    media: Vec<Media>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TwitterError {
    #[error(
        "Error getting tweet. {}: {}",
        raw.title,
        raw.detail.as_deref().unwrap_or("{details are unknown}")
    )]
    Service { raw: Error },

    #[error("Several errors occured: {raw_errors:#?}")]
    ServiceMany { raw_errors: Vec<Error> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test(tokio::test)]
    async fn manual_sandbox() {
        dotenvy::dotenv().ok();

        let cfg = crate::config::from_env_or_panic("TWITTER_");

        let client = Client::new(cfg, http::create_client());

        let tweet = client
            .get_tweet(TweetId(1608111355395211269))
            .await
            .unwrap();

        eprintln!("{tweet:#?}");
    }
}
