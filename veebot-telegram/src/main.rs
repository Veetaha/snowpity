use tracing::error;
use veebot_telegram::util::tracing_err;
use tracing::instrument;

#[tokio::main]
#[instrument(
    level = "error",
    fields(
        version = env!("VERGEN_BUILD_SEMVER"),
        git_commit = env!("VERGEN_GIT_SHA"),
    )
)]
async fn main() {
    if let Err(err) = try_main().await {
        error!(err = tracing_err(&err), "Exitting with an error...");
        std::process::exit(1);
    }
}

async fn try_main() -> veebot_telegram::Result {
    if let Err(_) = dotenv::dotenv() {
        eprintln!("Dotenv config was not found, ignoring this...")
    }

    {
        let config = veebot_telegram::LoggingConfig::load_or_panic();
        config.init_logging();
    }

    let config = veebot_telegram::Config::load_or_panic();

    veebot_telegram::run(config).await?;

    Ok(())
}
