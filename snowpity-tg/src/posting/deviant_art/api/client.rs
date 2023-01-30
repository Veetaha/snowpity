use crate::http;
use crate::posting::deviant_art::api::model::*;
use crate::posting::deviant_art::Config;
use crate::prelude::*;
use crate::Result;

http::def_url_base!(devianart_oembed, "https://backend.deviantart.com/oembed");

pub(crate) struct Client {
    http: http::Client,
}

impl Client {
    pub(crate) fn new(_cfg: Config, http: http::Client) -> Self {
        Self { http }
    }

    pub(crate) async fn get_oembed(&self, deviation: DeviationId) -> Result<GetOembedResponse> {
        self.http
            .get(devianart_oembed([]))
            .query(&[("url", deviation.to_canonical_url())])
            .read_json()
            .await
            .map_err(Into::into)
    }
}
