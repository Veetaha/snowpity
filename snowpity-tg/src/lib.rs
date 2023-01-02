mod config;
mod db;
mod display;
mod encoding;
mod error;
mod ftai;
mod http;
mod media_conv;
mod media_host;
mod observability;
mod sysinfo;
mod tg;

pub mod util;

pub use crate::error::*;
pub use config::*;
pub use observability::*;

#[allow(unused_imports)]
mod prelude {
    pub(crate) use crate::http::RequestBuilderExt;
    pub(crate) use crate::observability::logging::prelude::*;
    pub(crate) use crate::util::prelude::*;
    pub(crate) use snowpity_tg_macros::metered_db;
}

/// Run the telegram bot processing loop
pub async fn run(config: Config) -> Result<()> {
    let db = db::init(config.db).await?;

    let opts = tg::RunBotOptions {
        tg_cfg: config.tg,
        db,
        media_cfg: config.media,
    };

    tg::run_bot(opts).await?;

    Ok(())
}
