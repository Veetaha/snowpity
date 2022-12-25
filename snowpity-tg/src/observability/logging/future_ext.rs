use crate::prelude::*;
use async_trait::async_trait;
use easy_ext::ext;
use futures::prelude::*;
use std::time::Duration;

#[ext(FutureExt)]
#[async_trait]
pub(crate) impl<T, E, F> F
where
    F: Future<Output = Result<T, E>> + Send,
{
    async fn with_duration_log<'m>(self, msg: &'m str) -> F::Output {
        let (result, duration) = self.with_duration().await;
        let duration = tracing_duration(duration);
        match &result {
            Ok(_) => info!(result = "ok", duration, "{msg}"),
            Err(_) => warn!(result = "err", duration, "{msg}"),
        }
        result
    }

    async fn with_duration(self) -> (F::Output, Duration) {
        let start = std::time::Instant::now();
        let result = self.await;
        let elapsed = start.elapsed();
        (result, elapsed)
    }

    async fn with_duration_ok(self) -> Result<(T, Duration), E> {
        let start = std::time::Instant::now();
        let result = self.await?;
        let elapsed = start.elapsed();
        Ok((result, elapsed))
    }
}
