use crate::{err_ctx, DbError, Result};
use duplicate::duplicate;
use easy_ext::ext;
use sqlx::postgres::types::PgInterval;
use std::fmt;
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

trait DbReprMapping {
    type DbRepr;
}


/// Extension traits for types that can't be losslessly converted to database repr.
/// This is mostly conveniently only for mapping the error type to crate's [`DbError`].
pub(crate) trait IntoDbOrErr: DbReprMapping {
    fn into_db_or_err(self) -> Result<Self::DbRepr>;
}

pub(crate) trait IntoDb {
    type Db;

    fn into_db(self) -> Self::Db;
}

pub(crate) trait FromDbOrPanic: IntoDb {
    fn from_db_or_panic(val: Self::Db) -> Self;
}

pub(crate) trait FromDbOrErr<D>: Sized {
    type Err: std::error::Error + Send + Sync + 'static;

    fn from_db_or_err(val: D) -> Result<Self, Self::Err>;
}

pub(crate) trait IntoAppOrPanic<A> {
    fn into_app_or_panic(self) -> A;
}


impl<A: IntoDbOrErr<D>, D> IntoDbOrErr<Option<D>> for Option<A> {
    fn into_db_or_err(self) -> Result<Option<D>> {
        self.map(<_>::into_db_or_err).transpose()
    }
}

impl IntoDbOrErr<PgInterval> for Duration {
    fn into_db_or_err(self) -> Result<PgInterval> {
        self.try_into()
            .map_err(err_ctx!(DbError::InvalidDuration { duration: self }))
    }
}

impl<A: FromDbOrPanic<D>, D> IntoAppOrPanic<A> for D {
    fn into_app_or_panic(self) -> A {
        A::from_db_or_panic(self)
    }
}

impl<D, T> FromDbOrPanic<D> for T
where
    D: Clone + fmt::Debug,
    T: FromDbOrErr<D>,
{
    fn from_db_or_panic(val: D) -> Self {
        Self::from_db_or_err(val.clone()).unwrap_or_else(|err| {
            let ty = std::any::type_name::<Self>();
            panic!("Invalid {ty} in database: {val:?}\nError: {err:#?}")
        })
    }
}

duplicate! {
    [Ty; [ChatId]; [UserId]]

    impl IntoDb<String> for Ty {
        fn into_db(self) -> String {
            self.to_string()
        }
    }

    impl FromDbOrErr<String> for Ty {
        type Err = std::num::ParseIntError;

        fn from_db_or_err(str: String) -> Result<Ty, Self::Err> {
            str.parse().map(Ty)
        }
    }
}

impl<A: FromDbOrPanic<D>, D> FromDbOrPanic<Option<D>> for Option<A> {
    fn from_db_or_panic(val: Option<D>) -> Option<A> {
        val.map(<_>::from_db_or_panic)
    }
}

impl FromDbOrPanic<PgInterval> for Duration {
    fn from_db_or_panic(val: PgInterval) -> Self {
        assert_eq!(
            (val.months, val.days),
            (0, 0),
            "months and days are not supported"
        );
        Self::from_micros(val.microseconds.try_into().unwrap())
    }
}

impl IntoDb<String> for censy::TemplatePhrase {
    fn into_db(self) -> String {
        self.into_string()
    }
}

impl FromDbOrErr<String> for censy::TemplatePhrase {
    type Err = censy::TemplatePhraseError;

    fn from_db_or_err(val: String) -> Result<Self, Self::Err> {
        Self::new(&val)
    }
}
