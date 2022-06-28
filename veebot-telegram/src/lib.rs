mod config;
mod db;
#[allow(unused)]
mod derpibooru;
mod error;
mod ftai;
mod tg;

pub mod util;

pub use crate::error::*;
pub use config::*;

/// Run the telegram bot processing loop
pub async fn run(config: Config) -> Result<()> {
    let repo = db::init(config.db).await?;

    tg::run_bot(repo, config.tg).await?;

    Ok(())
}
