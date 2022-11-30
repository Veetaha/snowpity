use easy_ext::ext;

pub(crate) type PgQuery<'a> = sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>;

#[ext(ErrorExt)]
pub(crate) impl sqlx::Error {
    fn is_constraint_violation(&self, constraint: &str) -> bool {
        self.as_database_error()
            .map(|err| err.constraint() == Some(constraint))
            .unwrap_or(false)
    }
}
