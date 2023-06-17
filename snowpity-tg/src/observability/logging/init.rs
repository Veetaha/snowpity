use crate::config::from_env_or_panic;
use crate::observability::GLOBAL_LABELS;
use crate::prelude::*;
use serde::Deserialize;
use serde_with::serde_as;
use std::collections::HashMap;
use std::ops::Deref;
use tracing_subscriber::prelude::*;

pub struct LoggingTask {
    task: tokio::task::JoinHandle<()>,
    controller: tracing_loki::BackgroundTaskController,
}

impl LoggingTask {
    pub async fn shutdown(self) {
        info!("Waiting for the logging task to finish nicely...");

        let ((), duration) = self.controller.shutdown().with_duration().await;

        eprintln!(
            "Stopped logging task in {:.2?}: {:?}",
            duration,
            self.task.await
        );
    }
}

pub fn init_logging() -> LoggingTask {
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

    fn init_logging(self) -> LoggingTask {
        let env_filter = tracing_subscriber::EnvFilter::from_env("TG_BOT_LOG");

        let fmt = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(std::env::var("COLORS").as_deref() != Ok("0"))
            .pretty();

        let additional_labels = GLOBAL_LABELS.iter().chain(&[("source", "snowpity-tg")]);

        let mut labels = self.tg_bot_log_labels;
        labels.extend(additional_labels.map(|(k, v)| ((*k).to_owned(), (*v).to_owned())));

        let (loki, controller, task) = labels
            .into_iter()
            .fold(tracing_loki::builder(), |builder, (key, value)| {
                builder.label(key, value).unwrap()
            })
            .build_controller_url(self.loki_url)
            .unwrap();

        let task = tokio::spawn(task);

        tracing_subscriber::registry()
            .with(fmt)
            .with(loki)
            .with(env_filter)
            .with(tracing_error::ErrorLayer::default())
            .init();

        init_panic_hook();

        LoggingTask { task, controller }
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
