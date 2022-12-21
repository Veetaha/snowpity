mod config;
mod db;
mod derpi;
mod error;
mod ftai;
mod media;
mod observability;
mod sysinfo;
mod tg;

pub mod util;

pub use crate::error::*;
pub use config::*;
pub use observability::*;

mod prelude {
    pub(crate) use crate::util::prelude::*;
    pub(crate) use crate::observability::logging::prelude::*;
}


/// Run the telegram bot processing loop
pub async fn run(config: Config) -> Result<()> {
    let db = db::init(config.db).await?;
    tg::run_bot(config.tg, config.derpi, db).await?;

    Ok(())
}
