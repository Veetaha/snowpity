pub(crate) mod logging;
pub(crate) mod metrics;

pub use self::logging::{init_logging, tracing_err};
pub use self::metrics::init_metrics;

const GLOBAL_LABELS: &[(&str, &str)] = &[
    ("app_version", env!("VERGEN_BUILD_SEMVER")),
    ("app_git_commit", env!("VERGEN_GIT_SHA")),
];
