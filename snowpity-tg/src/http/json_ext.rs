use super::HttpClientError;
use crate::prelude::*;
use crate::{err, Result};
use async_trait::async_trait;
use easy_ext::ext;
use reqwest_middleware::RequestBuilder;
use serde::{de::DeserializeOwned, Serialize};

#[ext(RequestBuilderJsonExt)]
#[async_trait]
pub(crate) impl RequestBuilder {
    async fn send_and_read_json<Req: Serialize + Send + Sync, Res: DeserializeOwned>(
        self,
        req: Req,
    ) -> Result<Res> {
        self.json(&req).read_json().await
    }

    async fn read_json<Res: DeserializeOwned>(self) -> Result<Res> {
        let bytes = self.read_bytes().await?;

        serde_json::from_slice(&bytes).map_err(|err| {
            match std::str::from_utf8(&bytes) {
                Ok(response_body) => warn!(%response_body, "Bad JSON response"),
                Err(utf8_decode_err) => warn!(
                    response_body = ?bytes,
                    ?utf8_decode_err,
                    "Bad JSON response"
                ),
            };
            err!(HttpClientError::UnexpectedResponseJsonShape { source: err })
        })
    }
}
