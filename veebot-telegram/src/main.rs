use tracing::error;
use veebot_telegram::util::tracing_err;

#[tokio::main]
async fn main() {
    if let Err(_) = dotenv::dotenv() {
        eprintln!("Dotenv config was not found, ignoring this...")
    }

    veebot_telegram::LoggingConfig::load_or_panic().init_logging();

    if let Err(err) = try_main().await {
        error!(err = tracing_err(&err), "Exitting with an error...");
        std::process::exit(1);
    }
}

async fn try_main() -> veebot_telegram::Result {
    let config = veebot_telegram::Config::load_or_panic();

    veebot_telegram::run(config).await?;

    Ok(())
}
