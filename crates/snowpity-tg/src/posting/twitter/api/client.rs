use crate::posting::twitter::api::model::*;
use crate::posting::twitter::Config;
use crate::prelude::*;
use crate::Result;
use crate::{err_ctx, util};

util::url::def!(twitter_api, "https://api.twitter.com/2");

pub(crate) struct Client {}

impl Client {
    // The config is used at the start of the program to initialize twitter_scraper lib
    pub(crate) fn new(_: Config) -> Self {
        Self {}
    }

    pub(crate) async fn get_tweet(&self, id: TweetId) -> Result<Tweet> {
        let task = || async {
            util::tokio::spawn_blocking(move || twitter_scraper::get_tweet(&id.to_string()))
                .await
                .map(|tweet| Tweet::from_raw(id, tweet))
        };

        // TODO: figure out how to properly differentiate between retryable
        // and non-retryable errors here
        util::retry::retry_http(task, |_err| true)
            .await
            .map_err(err_ctx!(TwitterError::Service {}))
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TwitterError {
    #[error("Error getting tweet.")]
    Service { source: twitter_scraper::Error },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test(tokio::test)]
    #[ignore]
    async fn manual_sandbox() {
        let _ = dotenvy::dotenv();

        let cfg: Config = crate::config::from_env_or_panic("TWITTER_");

        twitter_scraper::initialize(&cfg.cookies);

        let client = Client::new(cfg);

        let tweet = client
            .get_tweet(TweetId::from_raw(1609634286050623492))
            .await
            .unwrap();

        eprintln!("{tweet:#?}");
    }
}
