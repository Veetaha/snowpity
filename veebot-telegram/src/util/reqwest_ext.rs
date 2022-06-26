use crate::{err_ctx, err_val, HttpError, Result};
use async_trait::async_trait;
use bytes::Bytes;
use easy_ext::ext;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::{debug, warn};

pub(crate) fn create_http_client() -> reqwest::Client {
    teloxide::net::client_from_env()
}

#[ext(ReqwestBuilderExt)]
#[async_trait]
pub(crate) impl reqwest::RequestBuilder {
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
                Ok(response_body) => warn!(response_body, "Bad JSON response"),
                Err(utf8_decode_err) => warn!(
                    response_body = ?bytes,
                    ?utf8_decode_err,
                    "Bad JSON response"
                ),
            };
            err_val!(HttpError::UnexpectedResponseJsonShape { source: err })
        })
    }

    async fn read_bytes(self) -> Result<Bytes> {
        let request = self
            // XXX: important for derpibooru (otherwise it responds with an html capcha page)
            .header("User-Agent", "Telegram Bot made by Veetaha");

        debug!(?request, "sending HTTP request");

        let res = request
            .send()
            .await
            .map_err(err_ctx!(HttpError::SendRequest))?;

        let status = res.status();

        if status.is_client_error() || status.is_server_error() {
            let body = match res.text().await {
                Ok(it) => it,
                Err(err) => format!("Could not collect the error response body text: {}", err),
            };

            return Err(err_val!(HttpError::BadResponseStatusCode { status, body }));
        }

        res.bytes().await.map_err(err_ctx!(HttpError::ReadResponse))
    }
}
