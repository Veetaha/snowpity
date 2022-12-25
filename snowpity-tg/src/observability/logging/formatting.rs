use std::fmt;
use std::time::Duration;

#[must_use]
pub fn tracing_err<'a, E: std::error::Error + 'static>(err: &'a E) -> impl tracing::Value + 'a {
    err as &dyn std::error::Error
}

pub(crate) fn tracing_duration(duration: Duration) -> impl tracing::Value {
    tracing::field::display(TracingDuration(duration))
}

struct TracingDuration(Duration);

impl fmt::Display for TracingDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2?}", self.0)
    }
}
