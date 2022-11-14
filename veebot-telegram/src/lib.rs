mod config;
mod db;
#[allow(unused)]
mod derpibooru;
mod error;
mod ftai;
mod tg;
mod sysinfo;

pub mod util;

pub use crate::error::*;
pub use config::*;

/// Run the telegram bot processing loop
pub async fn run(config: Config) -> Result<()> {
    let _ = db::init(config.db);

    tg::run_bot(config.tg).await?;

    Ok(())
}
