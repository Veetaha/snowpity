use crate::{PgQuery, PgQueryAs, PgQueryScalar};
use easy_ext::ext;
use sqlx::postgres::{PgArguments, PgRow};
use sqlx::{AssertSqlSafe, IntoArguments, SqlSafeStr, SqlStr};

pub mod expr {
    use sea_query::{Expr, ExprTrait, Func, Iden};

    #[derive(Iden)]
    struct Now;

    #[derive(Iden)]
    struct Timestamp;

    pub fn timestamp_now() -> Expr {
        Expr::from(Func::cust(Now)).cast_as(Timestamp)
    }
}

#[macro_export]
macro_rules! expr_vec {
    ($($value:expr,)* $(,)?) => {
        vec![
            $($crate::imp::sea_query::Expr::from($value),)*
        ]
    };
}

/// A query built by [`sea_query`], ready to be converted to one of the [`sqlx`]
/// query types.
pub struct SqlxQuery {
    sql: SqlStr,
    args: PgArguments,
}

#[ext(SqlxBinderExt)]
pub impl<T: sea_query_sqlx::SqlxBinder> T {
    fn to_sqlx(&self) -> SqlxQuery {
        let (sql, args) = self.build_sqlx(sea_query::PostgresQueryBuilder);
        SqlxQuery {
            // `sea_query` passes all values as bind parameters,
            // so the SQL string contains no user input.
            sql: AssertSqlSafe(sql).into_sql_str(),
            args: args.into_arguments(),
        }
    }
}

impl SqlxQuery {
    pub fn query(self) -> PgQuery<'static> {
        sqlx::query_with(self.sql, self.args)
    }

    pub fn query_as<O>(self) -> PgQueryAs<'static, O>
    where
        O: for<'r> sqlx::FromRow<'r, PgRow>,
    {
        sqlx::query_as_with(self.sql, self.args)
    }

    pub fn query_scalar<O>(self) -> PgQueryScalar<'static, O>
    where
        (O,): for<'r> sqlx::FromRow<'r, PgRow>,
    {
        sqlx::query_scalar_with(self.sql, self.args)
    }
}
