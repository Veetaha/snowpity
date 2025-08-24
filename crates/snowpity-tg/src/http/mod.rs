mod basic_ext;
mod json_ext;

use crate::prelude::*;
use async_trait::async_trait;
use reqwest_middleware::RequestBuilder;
use reqwest_retry::policies::{ExponentialBackoff, ExponentialBackoffTimed};
use reqwest_retry::RetryTransientMiddleware;
use std::time::{Duration, Instant};

pub(crate) mod prelude {
    pub(crate) use super::basic_ext::{RequestBuilderBasicExt, ResponseBasicExt};
    pub(crate) use super::json_ext::RequestBuilderJsonExt;
}

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

pub(crate) fn default_retry_policy() -> ExponentialBackoffTimed {
    // Retry exponentially increasing intervals between attempts.
    ExponentialBackoff::builder()
        .base(2)
        .retry_bounds(Duration::from_millis(100), Duration::from_secs(2))
        .build_with_total_retry_duration(Duration::from_secs(10))
}

pub(crate) fn create_client() -> Client {
    reqwest_middleware::ClientBuilder::new(teloxide::net::client_from_env())
        .with(OutermostObservingMiddleware)
        .with(RetryTransientMiddleware::new_with_policy(
            default_retry_policy(),
        ))
        .with(InnermostObservingMiddleware)
        .with_init(|request_builder: RequestBuilder| {
            request_builder.header(
                // XXX: this header important for derpibooru,
                // otherwise it responds with an html capcha page
                "User-Agent",
                concat!(
                    "SnowpityTelegramBot/",
                    env!("CARGO_PKG_VERSION"),
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
        extensions: &mut http::Extensions,
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
        extensions: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let (result, duration) =
            measure_request(http_request_duration_seconds, request, extensions, next)
                .with_duration()
                .await;

        let duration = tracing_duration(duration);

        let response = match &result {
            Ok(response) => response,
            Err(err) => {
                error!(duration, err = tracing_err(err), "Network request failed");
                return result;
            }
        };

        let status = response.status();

        let Err(err) = response.error_for_status_ref() else {
            info!(duration, %status, "Network request succeeded");
            return result;
        };

        warn!(
            err = tracing_err(&err),
            duration,
            %status,
            "Network request failed (error status)"
        );

        result
    }
}

async fn measure_request(
    histogram: fn(HttpResponseLabels) -> metrics::Histogram,
    request: reqwest::Request,
    extensions: &mut http::Extensions,
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

/// Errors at the layer of the HTTP API
#[derive(Debug, thiserror::Error)]
pub(crate) enum HttpClientError {
    #[error("HTTP request failed")]
    Request { source: reqwest_middleware::Error },

    #[error("Failed to read HTTP response")]
    ReadPayload { source: reqwest_middleware::Error },

    #[error("HTTP request has failed (HTTP status code: {status}):\n{body}")]
    BadResponseStatusCode {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Received an unexpected response JSON object")]
    UnexpectedResponseJsonShape { source: serde_json::Error },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test(tokio::test)]
    #[ignore]
    async fn manual_sandbox() {
        let url = "https://derpicdn.net/img/view/2018/10/19/1860230.mp4";

        let http = create_client();
        let response = http.head(url).send().await.unwrap();

        // dbg!(response.chunk().await);

        // dbg!(response.());

        dbg!(response.content_length());
    }
}
