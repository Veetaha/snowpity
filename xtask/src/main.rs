use tracing::error;
use tracing_subscriber::prelude::*;

fn main() {
    if let Err(err) = try_main() {
        error!("Exitting with an error...\n{err:?}");
    }
}

fn try_main() -> anyhow::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true).pretty();

    let filter = tracing_subscriber::EnvFilter::new("debug");

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter)
        .init();

    xtask::run()?;

    Ok(())
}
