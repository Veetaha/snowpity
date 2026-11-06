use crate::Result;
use crate::posting::{TgFileMeta, derpilike::api::MediaId};
use crate::prelude::*;
use sqlx::{AssertSqlSafe, SqlSafeStr, SqlStr};
use sqlx_bat::{TryIntoDb, prelude::*};
use std::sync::Arc;

pub(crate) struct BlobCacheRepo {
    db: sqlx::PgPool,
    insert_query: SqlStr,
    select_query: SqlStr,
}

#[derive(sqlx::FromRow)]
struct Record {
    tg_file_id: String,
    tg_file_kind: i16,
}

impl BlobCacheRepo {
    pub(crate) fn new(db: sqlx::PgPool, table_name: &'static str) -> Self {
        let table = format!("tg_{table_name}_blob_cache");

        // The table name is a static string from the code
        let sql = |query: String| AssertSqlSafe(Arc::<str>::from(query)).into_sql_str();

        Self {
            db,
            insert_query: sql(format!(
                "insert into {table} (media_id, tg_file_id, tg_file_kind) values ($1, $2, $3)"
            )),
            select_query: sql(format!(
                "select tg_file_id, tg_file_kind from {table} where media_id = $1"
            )),
        }
    }

    #[metered_db]
    pub(crate) async fn set(&self, media_id: MediaId, tg_file: TgFileMeta) -> Result {
        sqlx::query(self.insert_query.clone())
            .bind(media_id.try_into_db()?)
            .bind(tg_file.id)
            .bind(tg_file.kind.try_into_db()?)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    #[metered_db]
    pub(crate) async fn get(&self, media_id: MediaId) -> Result<Option<TgFileMeta>> {
        sqlx::query_as(self.select_query.clone())
            .bind(media_id.try_into_db()?)
            .fetch_optional(&self.db)
            .await?
            .map(
                |Record {
                     tg_file_id,
                     tg_file_kind,
                 }| {
                    Ok(TgFileMeta {
                        id: tg_file_id,
                        kind: tg_file_kind.try_into_app()?,
                    })
                },
            )
            .transpose()
    }
}
