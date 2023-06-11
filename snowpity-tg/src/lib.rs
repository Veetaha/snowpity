mod config;
mod db;
mod display;
mod encoding;
mod error;
mod ftai;
mod http;
mod media_conv;
mod observability;
mod posting;
mod sysinfo;
mod temp_file;
mod tg;
mod url;

pub mod util;

pub use crate::error::*;
pub use config::*;
pub use observability::*;

#[allow(unused_imports)]
mod prelude {
    pub(crate) use crate::error::prelude::*;
    pub(crate) use crate::http::prelude::*;
    pub(crate) use crate::observability::logging::prelude::*;
    pub(crate) use crate::temp_file::NamedTempFileExt;
    pub(crate) use crate::url::UrlExt;
    pub(crate) use crate::util::prelude::*;
    pub(crate) use snowpity_tg_macros::metered_db;
}

/// Run the telegram bot processing loop
pub async fn run(config: Config) -> Result<()> {
    let db = db::init(config.db).await?;

    let opts = tg::RunBotOptions {
        tg_cfg: config.tg,
        posting_cfg: config.posting,
        db,
    };

    tg::run_bot(opts).await?;

    Ok(())
}
