mod config;
mod constraints;
mod error;

mod tg_chat;

use crate::{err_ctx, Result};
use sqlx::prelude::*;

pub(crate) use {config::*, error::*, tg_chat::*};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

metrics_bat::histograms! {
    /// Database query duration in seconds
    pub(crate) db_query_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;
}

pub(crate) struct Repo {
    pub(crate) tg_chat: TgChatRepo,
}

pub(crate) async fn init(config: Config) -> Result<Repo> {
    let mut connect_options = config.url.as_str().parse::<PgConnectOptions>()?;

    connect_options.log_statements(log::LevelFilter::Debug);

    let db = PgPoolOptions::new()
        .max_connections(config.pool_size)
        // Verify that the connection is working early.
        // The connection created here can also be reused by the migrations down the road.
        // The default idle timeout should be enough for that.
        .connect_with(connect_options)
        .await
        .map_err(err_ctx!(DbError::Connect))?;

    sqlx::migrate!()
        .run(&db)
        .await
        .map_err(err_ctx!(DbError::Migrate))?;

    // Validate that our constraint names in code are fresh
    constraints::validate(db.clone()).await;

    Ok(Repo {
        tg_chat: TgChatRepo::new(db),
    })
}
