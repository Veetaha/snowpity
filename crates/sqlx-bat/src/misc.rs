use easy_ext::ext;

pub type PgQuery<'a> = sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>;

pub type PgQueryAs<'a, O> =
    sqlx::query::QueryAs<'a, sqlx::Postgres, O, sqlx::postgres::PgArguments>;

pub type PgQueryScalar<'a, O> =
    sqlx::query::QueryScalar<'a, sqlx::Postgres, O, sqlx::postgres::PgArguments>;

#[ext(ErrorExt)]
pub impl sqlx::Error {
    fn is_constraint_violation(&self, constraint: &str) -> bool {
        self.as_database_error()
            .map(|err| err.constraint() == Some(constraint))
            .unwrap_or(false)
    }
}
