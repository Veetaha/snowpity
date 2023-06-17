use crate::{http, util};
use crate::posting::deviant_art::api::model::*;
use crate::posting::deviant_art::Config;
use crate::prelude::*;
use crate::Result;

util::url::def!(backend_deviantart_com, "https://backend.deviantart.com");

pub(crate) struct Client {
    http: http::Client,
}

impl Client {
    pub(crate) fn new(_cfg: Config, http: http::Client) -> Self {
        Self { http }
    }

    pub(crate) async fn get_oembed(&self, deviation: DeviationId) -> Result<GetOembedResponse> {
        self.http
            .get(backend_deviantart_com(["oembed"]))
            .query(&[("url", deviation.to_canonical_url())])
            .read_json()
            .await
            .map_err(Into::into)
    }
}
