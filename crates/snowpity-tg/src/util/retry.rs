use crate::prelude::*;
use crate::Result;
use retry_policies::{RetryDecision, RetryPolicy};
use std::future::Future;
use chrono::prelude::*;

/// We already have an existing http client with retries set up in this crate.
/// However, this function is required to retry the HTTP operations that are
/// done outside of Rust (e.g. in the Go code of `twitter-scraper`).
pub(crate) async fn retry_http<T, E, Fut>(f: impl Fn() -> Fut, is_retryable: impl Fn(&E) -> bool) -> Fut::Output
where
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error,
{
    let policy = crate::http::default_retry_policy();
    let mut attempt = 0;
    loop {
        let err = match f().await {
            Ok(output) => {
                if attempt > 0 {
                    warn!(%attempt, "HTTP request succceded after a retry");
                }
                return Ok(output)
            }
            Err(err) => err,
        };

        if !is_retryable(&err) {
            if attempt > 0 {
                warn!(%attempt, "HTTP request failed with a non-retryable error after a retry");
            }
            return Err(err);
        }

        let execute_after = match policy.should_retry(attempt) {
            RetryDecision::Retry { execute_after } => execute_after,
            RetryDecision::DoNotRetry => {
                warn!(%attempt, "Giving up retrying HTTP request");
                return Err(err);
            }
        };

        let duration = (execute_after.signed_duration_since(Utc::now()))
            .to_std()
            .unwrap_or_else(|err| {
                warn!(
                    err = tracing_err(&err),
                    %execute_after,
                    "Retry policy returned a negative duration, retrying immediately"
                );
                std::time::Duration::ZERO
            });

        // Sleep the requested amount before we try again.
        warn!(
            %attempt,
            duration = format_args!("{duration:.2?}"),
            "Sleeping before the next attempt",
        );

        tokio::time::sleep(duration).await;

        attempt += 1;
    }
}
