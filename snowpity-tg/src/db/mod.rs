mod config;
mod conv;
mod tg_media_cache;

use crate::{err_ctx, DbError, Result};
use migration::{Migrator, MigratorTrait};

pub(crate) use config::*;
pub(crate) use tg_media_cache::*;

metrics_bat::labels! {
    DbQueryLabels { sql, result }
}

metrics_bat::histograms! {
    /// Database query duration in seconds
    db_query_duration_seconds = crate::metrics::DEFAULT_DURATION_BUCKETS;
}

pub(crate) struct Repo {
    pub(crate) media_cache: TgMediaCacheRepo,
}

pub(crate) async fn init(cfg: Config) -> Result<Repo> {
    let mut opts = sea_orm::ConnectOptions::new(cfg.url.into());
    opts.max_connections(cfg.pool_size);

    // Verify that the connection is working early.
    // The connection created here can also be reused by the migrations down the road.
    // The default idle timeout should be enough for that.
    let mut db = sea_orm::Database::connect(opts)
        .await
        .map_err(err_ctx!(DbError::Connect))?;

    db.set_metric_callback(|metric| {
        db_query_duration_seconds(DbQueryLabels {
            sql: metric.statement.sql.clone(),
            result: if metric.failed { "err" } else { "ok" },
        })
        .record(metric.elapsed);
    });

    Migrator::up(&db, None)
        .await
        .map_err(err_ctx!(DbError::Migrate))?;

    Ok(Repo {
        media_cache: TgMediaCacheRepo::new(db),
    })
}
