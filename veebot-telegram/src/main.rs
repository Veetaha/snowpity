use std::env;
use tracing::error;
use tracing_subscriber::prelude::*;
use veebot_telegram::util::tracing_err;

#[tokio::main]
async fn main() {
    if let Err(err) = try_main().await {
        error!(err = tracing_err(&err), "Exitting with an error...");
    }
}

async fn try_main() -> veebot_telegram::Result {
    if let Err(_) = dotenv::dotenv() {
        eprintln!("Dotenv config was not found, ignoring this...")
    }

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(env::var("COLORS").as_deref() != Ok("0"))
        .pretty();

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(tracing_subscriber::EnvFilter::from_env("VEEBOT_LOG"))
        .init();

    let config: veebot_telegram::Config =
        envy::from_env().expect("BUG: couldn't parse config from env variables");

    veebot_telegram::run(config).await?;

    Ok(())
}
