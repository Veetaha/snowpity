mod config;
mod db;
mod derpi;
mod error;
mod ftai;
mod media;
mod sysinfo;
mod tg;
mod metrics;

pub mod util;

pub use crate::error::*;
pub use config::*;

/// Run the telegram bot processing loop
pub async fn run(config: Config) -> Result<()> {
    let db = db::init(config.db).await?;
    tg::run_bot(config.tg, config.derpi, db).await?;

    Ok(())
}
