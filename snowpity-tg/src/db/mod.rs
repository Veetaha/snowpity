mod config;

mod tg_chats;
mod tg_media_cache;

use crate::{err_ctx, DbError, Result};

pub(crate) use config::*;
use sqlx::postgres::PgPoolOptions;
pub(crate) use tg_media_cache::*;

metrics_bat::histograms! {
    /// Database query duration in seconds
    db_query_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;
}

pub(crate) struct Repo {
    pub(crate) tg_media_cache: TgMediaCacheRepo,
    // pub(crate) tg_chats: TgChatsRepo,
    // pub(crate) tg_chat_banned_words: TgChatBannedWordsRepo,
}

pub(crate) async fn init(config: Config) -> Result<Repo> {
    let db = PgPoolOptions::new()
        .max_connections(config.pool_size)
        // Verify that the connection is working early.
        // The connection created here can also be reused by the migrations down the road.
        // The default idle timeout should be enough for that.
        .connect(config.url.as_str())
        .await
        .map_err(err_ctx!(DbError::Connect))?;

    sqlx::migrate!()
        .run(&db)
        .await
        .map_err(err_ctx!(DbError::Migrate))?;

    // // Validate that our constraint names in code are fresh
    // db_constraints::DbConstraints::new(pool.clone())
    //     .validate()
    //     .await;

    Ok(Repo {
        tg_media_cache: TgMediaCacheRepo::new(db),
    })
}
