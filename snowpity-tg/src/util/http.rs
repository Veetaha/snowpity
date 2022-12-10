use crate::util::prelude::*;
use crate::{err_ctx, err_val, HttpClientError, Result};
use async_trait::async_trait;
use bytes::Bytes;
use easy_ext::ext;
use reqwest_middleware::RequestBuilder;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use crate::metrics::def_metrics;

// def_metrics! {
//     /// Number of http requests
//     http_requests: IntCounter;

//     /// Number of http requests that failed
//     http_requests_errors: Int;
// }

pub type Client = reqwest_middleware::ClientWithMiddleware;

pub(crate) fn create_client() -> Client {
    // Retry up to 3 times with increasing intervals between attempts.
    let retry_policy = ExponentialBackoff::builder()
        .backoff_exponent(2)
        .retry_bounds(Duration::from_millis(100), Duration::from_secs(3))
        .build_with_total_retry_duration(Duration::from_secs(60));

    reqwest_middleware::ClientBuilder::new(teloxide::net::client_from_env())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .with(LoggingMiddleware)
        .with_init(|request_builder: RequestBuilder| {
            request_builder.header(
                // XXX: this header important for derpibooru,
                // otherwise it responds with an html capcha page
                "User-Agent",
                concat!(
                    "SnowpityTelegramBot/",
                    env!("VERGEN_BUILD_SEMVER"),
                    " (https://github.com/Veetaha/snowpity)",
                ),
            )
        })
        .build()
}

struct LoggingMiddleware;

#[async_trait]
impl reqwest_middleware::Middleware for LoggingMiddleware {
    async fn handle(
        &self,
        req: reqwest::Request,
        extensions: &mut task_local_extensions::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let method = req.method();
        let url = req.url();
        let scheme = req.version();
        let span = info_span!(
            "request",
            ?scheme,
            %method,
            %url,
        );
        async {
            debug!("Sending request");
            // http_requests().inc();

            let result = next.run(req, extensions).await;
            match &result {
                Ok(response) => {
                    if let Err(err) = response.error_for_status_ref() {
                        error!(err = tracing_err(&err), "Request failed (error status)");
                        // http_requests_errors().with_label_values()
                    }
                }
                Err(err) => {
                    error!(err = tracing_err(err), "Request failed");
                }
            }
            result
        }
        .instrument(span)
        .await
    }
}

#[ext(RequestBuilderExt)]
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
                Ok(response_body) => warn!(response_body, "Bad JSON response"),
                Err(utf8_decode_err) => warn!(
                    response_body = ?bytes,
                    ?utf8_decode_err,
                    "Bad JSON response"
                ),
            };
            err_val!(HttpClientError::UnexpectedResponseJsonShape { source: err })
        })
    }

    async fn read_bytes(self) -> Result<Bytes> {
        let res = self
            .send()
            .await
            .map_err(err_ctx!(HttpClientError::SendRequest))?;

        let status = res.status();

        if status.is_client_error() || status.is_server_error() {
            let body = match res.text().await {
                Ok(it) => it,
                Err(err) => format!("Could not collect the error response body text: {}", err),
            };

            return Err(err_val!(HttpClientError::BadResponseStatusCode { status, body }));
        }

        res.bytes().await.map_err(err_ctx!(HttpClientError::ReadResponse))
    }

    // async fn read_to_temp_file(self) -> Result<tempfile::TempPath> {
    //     let file =
    //         tempfile::NamedTempFile::new().map_err(err_ctx!(crate::IoError::CreateTempFile))?;

    //     let (file, path) = file.into_parts();
    //     let file = tokio::fs::File::from_std(file);

    //     self.read_to_file_handle(&mut file).await?;

    //     Ok(path)
    // }

    // async fn read_to_file_handle(self, file_handle: &mut tokio::fs::File) -> Result {
    //     let mut stream = self
    //         .send()
    //         .await
    //         .map_err(err_ctx!(HttpError::ReadResponse))?
    //         .bytes_stream();

    //     while let Some(chunk) = stream.next().await {
    //         let chunk = chunk.map_err(err_ctx!(HttpError::ReadResponse))?;
    //         file_handle
    //             .write_all(&chunk)
    //             .await
    //             .map_err(err_ctx!(HttpError::WriteToFile))?;
    //     }

    //     file_handle
    //         .flush()
    //         .await
    //         .map_err(err_ctx!(HttpError::FlushToFile))?;

    //     Ok(())
    // }
}
