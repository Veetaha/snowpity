use crate::prelude::*;
use crate::{err, err_ctx, Result};
use async_trait::async_trait;
use bytes::Bytes;
use easy_ext::ext;
use reqwest_middleware::RequestBuilder;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::{Duration, Instant};

macro_rules! def_url_base {
    ($vis:vis $ident:ident, $url:literal) => {
        $vis fn $ident<T: AsRef<str>>(segments: impl IntoIterator<Item = T>) -> ::url::Url {
            let mut url: ::url::Url = $url.parse().unwrap();
            url.path_segments_mut().unwrap().extend(segments);
            url
        }
    };
}

pub(crate) use def_url_base;

metrics_bat::labels! {
    HttpRequestLabels { version, method, host }
    HttpResponseLabels { version, method, host, status }
}

metrics_bat::histograms! {
    /// Duration of a single real http request. If there were retries, then these
    /// will appear as as separate observations.
    http_request_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;

    /// Same as `http_requests_duration_seconds` but covers the time it took to
    /// do retries of the request.
    http_request_effective_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;
}

pub type Client = reqwest_middleware::ClientWithMiddleware;

pub(crate) fn create_client() -> Client {
    // Retry up to 3 times with increasing intervals between attempts.
    let retry_policy = ExponentialBackoff::builder()
        .backoff_exponent(2)
        .retry_bounds(Duration::from_millis(100), Duration::from_secs(3))
        .build_with_total_retry_duration(Duration::from_secs(60));

    reqwest_middleware::ClientBuilder::new(teloxide::net::client_from_env())
        .with(OutermostObservingMiddleware)
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
            http_request_effective_duration_seconds,
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
        let (result, duration) =
            measure_request(http_request_duration_seconds, request, extensions, next)
                .with_duration()
                .await;

        let duration = tracing_duration(duration);

        match &result {
            Ok(response) => {
                let status = response.status();

                if let Err(err) = response.error_for_status_ref() {
                    warn!(
                        err = tracing_err(&err),
                        duration,
                        %status,
                        "Network request failed (error status)"
                    );
                } else {
                    info!(duration, %status, "Network request succeded");
                }
            }
            Err(err) => {
                error!(duration, err = tracing_err(err), "Network request failed");
            }
        };

        result
    }
}

async fn measure_request(
    histogram: fn(HttpResponseLabels) -> metrics::Histogram,
    request: reqwest::Request,
    extensions: &mut task_local_extensions::Extensions,
    next: reqwest_middleware::Next<'_>,
) -> reqwest_middleware::Result<reqwest::Response> {
    let labels = request_labels(&request);

    let start = Instant::now();
    let result = next.run(request, extensions).await;
    let elapsed = start.elapsed();

    let status = match &result {
        Ok(response) => response.status().to_string(),
        Err(_) => "{fatal}".to_owned(),
    };

    let labels = HttpResponseLabels {
        status,
        version: labels.version,
        method: labels.method,
        host: labels.host,
    };

    histogram(labels).record(elapsed);

    result
}

fn request_labels(request: &reqwest::Request) -> HttpRequestLabels {
    HttpRequestLabels {
        version: format!("{:?}", request.version()),
        method: request.method().to_string(),
        host: request.url().host_str().unwrap_or("{unknown}").to_owned(),
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

            return Err(err!(HttpClientError::BadResponseStatusCode {
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
    //     let mut file = tokio::fs::File::from_std(file);

    //     self.read_to_file_handle(&mut file).await?;

    //     Ok(path)
    // }

    // async fn read_to_file_handle(self, file_handle: &mut tokio::fs::File) -> Result {
    //     let mut stream = self
    //         .send()
    //         .await
    //         .map_err(err_ctx!(HttpClientError::ReadResponse))?
    //         .bytes_stream();

    //     while let Some(chunk) = stream.next().await {
    //         let chunk = chunk.map_err(err_ctx!(HttpClientError::ReadResponse))?;
    //         file_handle
    //             .write_all(&chunk)
    //             .await
    //             .map_err(err_ctx!(HttpClientError::WriteToFile))?;
    //     }

    //     file_handle
    //         .flush()
    //         .await
    //         .map_err(err_ctx!(HttpClientError::FlushToFile))?;

    //     Ok(())
    // }
}

/// Errors at the layer of the HTTP API
#[derive(Debug, thiserror::Error)]
pub(crate) enum HttpClientError {
    #[error("Failed to send an http request")]
    SendRequest { source: reqwest_middleware::Error },

    #[error("Failed to read http response")]
    ReadResponse { source: reqwest_middleware::Error },

    #[error("HTTP request has failed (http status code: {status}):\n{body}")]
    BadResponseStatusCode {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Received an unexpected response JSON object")]
    UnexpectedResponseJsonShape { source: serde_json::Error },
    // #[error("Failed to write bytes to a file")]
    // WriteToFile { source: std::io::Error },

    // #[error("Failed to flush bytes to a file")]
    // FlushToFile { source: std::io::Error },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test(tokio::test)]
    async fn manual_sandbox() {
        let url = "https://derpicdn.net/img/view/2018/10/19/1860230.mp4";

        let http = create_client();
        let response = http.head(url)
            .send()
            .await
            .unwrap();

        // dbg!(response.chunk().await);

        // dbg!(response.());

        dbg!(response.content_length());
    }
}
