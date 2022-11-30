// TODO: the database module will be used some day
#![allow(unused)]

mod db_constraints;
// mod tg_chat_banned_words;
// mod tg_chats;

use crate::{err_ctx, DbConfig, DbError, Result};
use dptree::di::DependencyMap;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

// pub(crate) use tg_chat_banned_words::*;
// pub(crate) use tg_chats::*;

pub(crate) struct Repo {
    // pub(crate) tg_chats: TgChatsRepo,
    // pub(crate) tg_chat_banned_words: TgChatBannedWordsRepo,
}

pub(crate) async fn init(config: DbConfig) -> Result<Repo> {
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
    db_constraints::DbConstraints::new(pool.clone())
        .validate()
        .await;

    Ok(Repo {
        // tg_chat_banned_words: TgChatBannedWordsRepo::new(pool.clone()),
        // tg_chats: TgChatsRepo::new(pool),
    })
}
