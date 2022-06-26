use crate::{err_ctx, DbError, Result};
use duplicate::duplicate;
use easy_ext::ext;
use sqlx::postgres::types::PgInterval;
use std::time::Duration;
use teloxide::types::{ChatId, UserId};

pub(crate) type PgQuery<'a> = sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>;

#[ext(ErrorExt)]
pub(crate) impl sqlx::Error {
    fn is_constraint_violation(&self, constraint: &str) -> bool {
        self.as_database_error()
            .map(|err| err.constraint() == Some(constraint))
            .unwrap_or(false)
    }
}

/// Extension traits for types that can't be losslessly converted to database repr.
/// This is mostly conveniently only for mapping the error type to crate's [`DbError`].
pub(crate) trait TryIntoDb<D> {
    fn try_into_db(self) -> Result<D>;
}

impl<A: TryIntoDb<D>, D> TryIntoDb<Option<D>> for Option<A> {
    fn try_into_db(self) -> Result<Option<D>> {
        self.map(<_>::try_into_db).transpose()
    }
}

impl TryIntoDb<PgInterval> for Duration {
    fn try_into_db(self) -> Result<PgInterval> {
        self.try_into()
            .map_err(err_ctx!(DbError::InvalidDuration { duration: self }))
    }
}

pub(crate) trait IntoDb<D> {
    fn into_db(self) -> D;
}

pub(crate) trait FromDb<D> {
    fn from_db(val: D) -> Self;
}

impl<A: FromDb<D>, D> FromDb<Option<D>> for Option<A> {
    fn from_db(val: Option<D>) -> Option<A> {
        val.map(<_>::from_db)
    }
}

impl FromDb<PgInterval> for Duration {
    fn from_db(val: PgInterval) -> Self {
        assert_eq!(
            (val.months, val.days),
            (0, 0),
            "months and days are not supported"
        );
        Self::from_micros(val.microseconds.try_into().unwrap())
    }
}

pub(crate) trait IntoApp<A> {
    fn into_app(self) -> A;
}

impl<A: FromDb<D>, D> IntoApp<A> for D {
    fn into_app(self) -> A {
        A::from_db(self)
    }
}

duplicate! {
    [Ty; [ChatId]; [UserId]]

    impl IntoDb<String> for Ty {
        fn into_db(self) -> String {
            self.to_string()
        }
    }

    impl FromDb<String> for Ty {
        fn from_db(str: String) -> Ty {
            let id = str
                .parse()
                .unwrap_or_else(|err| {
                    let ty = std::any::type_name::<Ty>();
                    panic!("Invalid {} in database: {:?}\n{:#?}", ty, str, err)
                });
            Ty(id)
        }
    }
}
