use crate::metrics::def_metrics;
use crate::util::prelude::*;
use crate::{err_ctx, err_val, HttpClientError, Result};
use async_trait::async_trait;
use bytes::Bytes;
use easy_ext::ext;
use prometheus::labels;
use reqwest_middleware::RequestBuilder;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

def_metrics! {
    /// Number of http requests partitioned by status codes and methods
    http_requests_total: IntCounterVec [version, method, host, status];

    /// Number of http requests that failed due to fatal error.
    /// Doesn't include 500 errors.
    http_requests_fatal_total: IntCounterVec [version, method, host];

    /// Time spent on http requests partitioned by status codes and methods
    [buckets: *prometheus::DEFAULT_BUCKETS]
    http_request_duration_seconds: HistogramVec [version, method, host];

    /// Same as `http_requests_time` but includes the time it took to retry the request.
    [buckets: *prometheus::DEFAULT_BUCKETS]
    http_request_effective_duration_seconds: HistogramVec [version, method, host];
}

pub type Client = reqwest_middleware::ClientWithMiddleware;

pub(crate) fn create_client() -> Client {
    // Retry up to 3 times with increasing intervals between attempts.
    let retry_policy = ExponentialBackoff::builder()
        .backoff_exponent(2)
        .retry_bounds(Duration::from_millis(100), Duration::from_secs(3))
        .build_with_total_retry_duration(Duration::from_secs(60));

    reqwest_middleware::ClientBuilder::new(teloxide::net::client_from_env())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .with(InnermostObservingMiddleware)
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

struct OutermostObservingMiddleware;

#[async_trait]
impl reqwest_middleware::Middleware for OutermostObservingMiddleware {
    async fn handle(
        &self,
        request: reqwest::Request,
        extensions: &mut task_local_extensions::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let span = info_span!(
            "request",
            version = ?request.version(),
            method = %request.method(),
            url = %request.url(),
        );

        measure_request(
            http_requests_effective_time(),
            http_requests::version,
            http_requests::method,
            http_requests::host,
            request,
            extensions,
            next,
        )
        .instrument(span)
        .await
    }
}

struct InnermostObservingMiddleware;

#[async_trait]
impl reqwest_middleware::Middleware for InnermostObservingMiddleware {
    async fn handle(
        &self,
        request: reqwest::Request,
        extensions: &mut task_local_extensions::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let method = request.method().to_string();
        let host = request.url().host_str().unwrap_or("{unknown}").to_owned();
        let version = format!("{:?}", request.version());

        debug!("Sending request");

        let result = measure_request(
            http_requests_time(),
            http_requests_time::version,
            http_requests_time::method,
            http_requests_time::host,
            request,
            extensions,
            next,
        )
        .await;

        match &result {
            Ok(response) => {
                let status = response.status();
                let labels = &labels! {
                    http_requests::version => version.as_str(),
                    http_requests::status => status.as_str(),
                    http_requests::method => method.as_str(),
                    http_requests::host => host.as_str(),
                };
                http_requests().with(labels).inc();

                if let Err(err) = response.error_for_status_ref() {
                    error!(err = tracing_err(&err), "Request failed (error status)");
                }
            }
            Err(err) => {
                let labels = &labels! {
                    http_requests_fatal::version => version.as_str(),
                    http_requests_fatal::method => method.as_str(),
                    http_requests_fatal::host => host.as_str(),
                };
                http_requests_fatal().with(labels).inc();
                error!(err = tracing_err(err), "Request failed");
            }
        }
        result
    }
}

async fn measure_request(
    histogram: &prometheus::HistogramVec,
    version_label: &str,
    method_label: &str,
    host_label: &str,
    request: reqwest::Request,
    extensions: &mut task_local_extensions::Extensions,
    next: reqwest_middleware::Next<'_>,
) -> reqwest_middleware::Result<reqwest::Response> {
    let version = format!("{:?}", request.version());
    let labels = &labels! {
        version_label => version.as_str(),
        method_label => request.method().as_str(),
        host_label => request.url().host_str().unwrap_or("{unknown}"),
    };
    let _guard = histogram.with(labels).start_timer();
    next.run(request, extensions).await
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

            return Err(err_val!(HttpClientError::BadResponseStatusCode {
                status,
                body
            }));
        }

        res.bytes()
            .await
            .map_err(err_ctx!(HttpClientError::ReadResponse))
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
