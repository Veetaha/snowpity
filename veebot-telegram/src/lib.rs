mod config;
mod db;
#[allow(unused)]
mod derpibooru;
mod error;
mod tg;

pub mod util;

pub use crate::error::*;
pub use config::*;

/// Run the telegram bot processing loop
pub async fn run(config: Config) -> Result<()> {
    let mut di = dptree::di::DependencyMap::new();

    db::init(&mut di, config.db).await?;

    tg::run_bot(di, config.tg).await?;

    Ok(())
}
