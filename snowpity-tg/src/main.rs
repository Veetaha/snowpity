use futures::prelude::*;
use snowpity_tg::tracing_err;
use std::panic::AssertUnwindSafe;
use std::process::ExitCode;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> ExitCode {
    if dotenvy::dotenv().is_err() {
        eprintln!("Dotenv config was not found, ignoring this...")
    }

    let logging_task = snowpity_tg::init_logging();

    snowpity_tg::init_metrics();

    let main_fut = AssertUnwindSafe(async {
        let result = try_main().await;

        result.map(|()| ExitCode::SUCCESS).unwrap_or_else(|err| {
            error!(err = tracing_err(&err), "Exitting with an error...");
            ExitCode::FAILURE
        })
    })
    .catch_unwind()
    .unwrap_or_else(|_| {
        error!("Exitting due to a panic...");
        ExitCode::FAILURE
    });

    let exit_code = if !cfg!(debug_assertions) {
        main_fut.await
    } else {
        // Don't wait for teloxide's shutdown logic when cancelling in debug mode.
        // That takes a lot of time for some reason:
        // https://github.com/teloxide/teloxide/issues/711
        tokio::select! {
            exit_code = main_fut => {
                info!("Main task has finished, exiting...");
                exit_code
            }
            () = abort_signal() => ExitCode::SUCCESS,
        }
    };

    logging_task.shutdown().await;

    exit_code
}

async fn try_main() -> snowpity_tg::Result {
    let config = snowpity_tg::Config::load_or_panic();
    snowpity_tg::run(config).await
}

async fn abort_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        warn!(
            err = tracing_err(&err),
            "Failed to wait for Ctrl+C, exiting..."
        );
    } else {
        info!("Ctrl+C received, exiting forcefully...");
    }
}
