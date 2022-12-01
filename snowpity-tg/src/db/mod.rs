mod config;
mod db_constraints;
// mod tg_chat_banned_words;
// mod tg_chats;

use crate::{err_ctx, DbError, Result};
use sqlx::postgres::PgPoolOptions;

pub(crate) use config::*;

pub(crate) struct Repo {}

pub(crate) async fn init(config: Config) -> Result<Repo> {
    let pool = PgPoolOptions::new()
        .max_connections(config.pool_size)
        // Verify that the connection is working early.
        // The connection created here can also be reused by the migrations down the road.
        // The default idle timeout should be enough for that.
        .connect(config.url.as_str())
        .await
        .map_err(err_ctx!(DbError::Connect))?;

    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .map_err(err_ctx!(DbError::Migrate))?;
    // Validate that our constraint names in code are fresh
    db_constraints::DbConstraints::new(pool.clone())
        .validate()
        .await;

    Ok(Repo {})
}
