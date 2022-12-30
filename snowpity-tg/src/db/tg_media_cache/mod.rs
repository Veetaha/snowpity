mod derpi;
mod twitter;

pub(crate) use derpi::*;
pub(crate) use twitter::*;

pub(crate) struct TgMediaCacheRepo {
    pub(crate) derpi: TgDerpiMediaCacheRepo,
    pub(crate) twitter: TgTwitterMediaCacheRepo,
}


impl TgMediaCacheRepo {
    pub(crate) fn new(db: sqlx::PgPool) -> Self {
        Self {
            derpi: TgDerpiMediaCacheRepo::new(db.clone()),
            twitter: TgTwitterMediaCacheRepo::new(db),
        }
    }
}
