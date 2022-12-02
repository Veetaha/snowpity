mod config;
mod entities;
mod tg_derpi_media_cache;

use crate::{err_ctx, DbError, Result};
use migration::{Migrator, MigratorTrait};

pub(crate) use config::*;

pub(crate) struct Repo {}

pub(crate) async fn init(cfg: Config) -> Result<Repo> {
    let mut opts = sea_orm::ConnectOptions::new(cfg.url.into());
    opts.max_connections(cfg.pool_size);

    // Verify that the connection is working early.
    // The connection created here can also be reused by the migrations down the road.
    // The default idle timeout should be enough for that.
    let conn = sea_orm::Database::connect(opts)
        .await
        .map_err(err_ctx!(DbError::Connect))?;

    Migrator::up(&conn, None)
        .await
        .map_err(err_ctx!(DbError::Migrate))?;

    Ok(Repo {})
}
