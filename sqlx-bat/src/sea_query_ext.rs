use crate::{PgQuery, PgQueryAs, PgQueryScalar};
use easy_ext::ext;
use sqlx::postgres::{PgArguments, PgRow};
use sqlx::IntoArguments;

pub mod expr {
    use sea_query::{Func, Iden, SimpleExpr};

    #[derive(Iden)]
    struct Now;

    #[derive(Iden)]
    struct Timestamp;

    pub fn timestamp_now() -> SimpleExpr {
        SimpleExpr::from(Func::cust(Now)).cast_as(Timestamp)
    }
}

#[macro_export]
macro_rules! simple_expr_vec {
    ($($value:expr,)* $(,)?) => {
        vec![
            $($crate::imp::sea_query::SimpleExpr::from($value),)*
        ]
    };
}

/// Unfortunately, [`sqlx`] query types are limited to borrowing the SQL string,
/// so we must have this intermediate wrapper, that is also borrowed by a unique
/// reference during the method call chain to let us keep it in scope while the
/// [`sqlx`] query is being used.
pub struct SqlxQuery {
    sql: String,
    args: Option<PgArguments>,
}

#[ext(SqlxBinderExt)]
pub impl<T: sea_query_binder::SqlxBinder> T {
    fn into_sqlx<'a>(&self) -> SqlxQuery {
        let (sql, args) = self.build_sqlx(sea_query::PostgresQueryBuilder);
        SqlxQuery {
            sql,
            args: Some(args.into_arguments()),
        }
    }
}

impl SqlxQuery {
    /// Convert this to [`PgQuery`].
    ///
    /// # Panics
    ///
    /// This method should be a consuming one, but due to the limitations of [`sqlx`],
    /// the check for the double call is done at runtime, so you must make sure
    /// not to use the [`SqlxQuery`] after calling this method.
    pub fn query(&mut self) -> PgQuery<'_> {
        let args = self.unwrap_args();
        sqlx::query_with(&self.sql, args)
    }

    /// Convert this to [`PgQueryAs`].
    ///
    /// # Panics
    ///
    /// This method should be a consuming one, but due to the limitations of [`sqlx`],
    /// the check for the double call is done at runtime, so you must make sure
    /// not to use the [`SqlxQuery`] after calling this method.
    pub fn query_as<O>(&mut self) -> PgQueryAs<'_, O>
    where
        O: for<'r> sqlx::FromRow<'r, PgRow>,
    {
        let args = self.unwrap_args();
        sqlx::query_as_with(&self.sql, args)
    }

    /// Convert this to [`PgQueryScalar`].
    ///
    /// # Panics
    ///
    /// This method should be a consuming one, but due to the limitations of [`sqlx`],
    /// the check for the double call is done at runtime, so you must make sure
    /// not to use the [`SqlxQuery`] after calling this method.
    pub fn query_scalar<O>(&mut self) -> PgQueryScalar<'_, O>
    where
        (O,): for<'r> sqlx::FromRow<'r, PgRow>,
    {
        let args = self.unwrap_args();
        sqlx::query_scalar_with(&self.sql, args)
    }

    fn unwrap_args(&mut self) -> PgArguments {
        self.args
            .take()
            .expect("BUG: it is allowed to build sqlx query only once")
    }
}
