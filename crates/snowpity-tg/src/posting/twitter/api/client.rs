use crate::posting::twitter::api::model::*;
use crate::posting::twitter::Config;
use crate::prelude::*;
use crate::{util, Result};

util::url::def!(pub(crate) fixvx_api, "https://api.vxtwitter.com/Twitter/status");

pub(crate) struct Client {
    client: crate::http::Client,
}

impl Client {
    // The config is used at the start of the program to initialize twitter_scraper lib
    pub(crate) fn new(_: Config) -> Self {
        Self {
            client: crate::http::create_client(),
        }
    }

    /// API docs: <https://github.com/dylanpdx/BetterTwitFix>
    pub(crate) async fn get_tweet(&self, id: TweetId) -> Result<Tweet> {
        let url = fixvx_api([&id.to_string()]);
        let tweet = self.client.get(url).read_json::<Tweet>().await?;
        Ok(tweet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test(tokio::test)]
    #[ignore]
    async fn manual_sandbox() {
        let _ = dotenvy::dotenv();

        let cfg: Config = crate::config::from_env_or_panic("TWITTER_");

        let client = Client::new(cfg);

        let tweet = client
            .get_tweet(TweetId::from_raw(1609634286050623492))
            .await
            .unwrap();

        eprintln!("{tweet:#?}");
    }
}
