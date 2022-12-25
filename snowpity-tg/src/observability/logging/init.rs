use crate::config::from_env_or_panic;
use crate::observability::GLOBAL_LABELS;
use crate::prelude::*;
use serde::Deserialize;
use serde_with::serde_as;
use std::collections::HashMap;
use std::ops::Deref;
use tracing_subscriber::prelude::*;

pub fn init_logging() -> tokio::task::JoinHandle<()> {
    LoggingConfig::load_or_panic().init_logging()
}

#[serde_as]
#[derive(Deserialize)]
struct LoggingConfig {
    loki_url: url::Url,
    #[serde_as(as = "serde_with::json::JsonString")]
    tg_bot_log_labels: HashMap<String, String>,
}

impl LoggingConfig {
    fn load_or_panic() -> LoggingConfig {
        from_env_or_panic("")
    }

    fn init_logging(self) -> tokio::task::JoinHandle<()> {
        let env_filter = tracing_subscriber::EnvFilter::from_env("TG_BOT_LOG");

        let fmt = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(std::env::var("COLORS").as_deref() != Ok("0"))
            .pretty();

        let additional_labels = GLOBAL_LABELS.iter().chain(&[("source", "snowpity-tg")]);

        let mut labels = self.tg_bot_log_labels;
        labels.extend(additional_labels.map(|(k, v)| ((*k).to_owned(), (*v).to_owned())));

        let (loki, task) = tracing_loki::layer(self.loki_url, labels, HashMap::new()).unwrap();

        let join_handle = tokio::spawn(task);

        tracing_subscriber::registry()
            .with(fmt)
            .with(loki)
            .with(env_filter)
            .with(tracing_error::ErrorLayer::default())
            .init();

        init_panic_hook();

        join_handle
    }
}

fn init_panic_hook() {
    let current_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        // It's super-important to call the default panic hook, otherwise
        // we may not see it in the logs at all, because the panic may
        // happen inside of `tracing` logging system itself.
        // See the footgun: https://github.com/rust-itertools/itertools/issues/667
        current_hook(panic_info);

        let backtrace = std::backtrace::Backtrace::capture();
        let location = panic_info.location().map(|location| {
            format!(
                "{}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            )
        });

        // If the panic message was formatted using interpolated values,
        // it will be a `String`. Otherwise, it will be a `&str`.
        let payload = panic_info.payload();
        let message = payload
            .downcast_ref::<String>()
            .map(<_>::deref)
            .or_else(|| payload.downcast_ref::<&str>().map(<_>::deref))
            .unwrap_or("<unknown>");

        let span_trace = tracing_error::SpanTrace::capture();

        error!(
            target: "panic",
            thread = std::thread::current().name(),
            location,
            span_trace = %span_trace,
            backtrace = format_args!("\n{backtrace}"),
            "{message}"
        );
    }));
}
