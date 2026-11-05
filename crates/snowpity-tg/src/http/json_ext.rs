use super::HttpClientError;
use crate::prelude::*;
use crate::{Result, err};
use async_trait::async_trait;
use easy_ext::ext;
use reqwest_middleware::RequestBuilder;
use serde::de::DeserializeOwned;

#[ext(RequestBuilderJsonExt)]
#[async_trait]
pub(crate) impl RequestBuilder {
    async fn read_json<Res: DeserializeOwned>(self) -> Result<Res> {
        let bytes = self.read_bytes().await?;
        let err = match serde_json::from_slice(&bytes) {
            Ok(res) => return Ok(res),
            Err(err) => err,
        };

        match std::str::from_utf8(&bytes) {
            Ok(response_body) => warn!(%response_body, "Bad JSON response"),
            Err(utf8_decode_err) => warn!(
                response_body = ?bytes,
                ?utf8_decode_err,
                "Bad JSON response (invalid UTF-8)"
            ),
        }

        Err(err!(HttpClientError::UnexpectedResponseJsonShape {
            source: err
        }))
    }
}
