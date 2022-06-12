mod db_constraints;
mod tg_chat_banned_patterns;
mod tg_chats;

use crate::{err_ctx, DbConfig, DbError, Result};
use dptree::di::DependencyMap;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

pub(crate) use tg_chat_banned_patterns::*;
pub(crate) use tg_chats::*;

pub(crate) struct Repo {
    pub(crate) tg_chats: TgChatsRepo,
    pub(crate) tg_chat_banned_patterns: TgChatBannedPatternsRepo,
}

pub(crate) async fn init(di: &mut DependencyMap, config: DbConfig) -> Result {
    let pool = PgPoolOptions::new()
        .max_connections(config.pool_size)
        // Verify that the connection is working early.
        // The connection created here can also be reused by the migrations down the road.
        // The default idle timeout should be enough for that.
        .connect(config.url.as_str())
        .await
        .map_err(err_ctx!(DbError::Connect))?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(err_ctx!(DbError::Migrate))?;

    // Validate that our constraint names in code are fresh
    db_constraints::DbConstraints::new(pool.clone()).validate();

    di.insert(Arc::new(Repo {
        tg_chat_banned_patterns: TgChatBannedPatternsRepo::new(pool.clone()),
        tg_chats: TgChatsRepo::new(pool),
    }));

    Ok(())
}
