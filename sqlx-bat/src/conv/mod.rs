mod impls;
mod std_impls;
mod teloxide_impls;

use crate::Result;
use std::fmt;

/// A type that has a database representation specified in [`DbRepresentable::DbRepr`]
pub trait DbRepresentable: fmt::Debug + Sized + Send + Sync + 'static {
    type DbRepr: fmt::Debug + Sized + Send + Sync + 'static;
}

/// A type that can't be losslessly converted to database repr.
/// Mostly convenient only for mapping the error type to crate's [`DbError`].
pub trait TryIntoDb: DbRepresentable {
    fn try_into_db(self) -> Result<Self::DbRepr>;
}

/// A type that can be losslessly converted to database repr.
pub trait IntoDb: DbRepresentable {
    fn into_db(self) -> Self::DbRepr;
}

/// A type, that can be losslessly converted from database repr.
pub trait TryFromDb: DbRepresentable {
    fn try_from_db(val: Self::DbRepr) -> Result<Self>;
}

/// Same as [`TryFromDb`], but represents a mirror side of the conversion.
/// It is automatically implemented for any type, that implements [`TryFromDb`].
pub trait TryIntoApp<A> {
    fn try_into_app(self) -> Result<A>;
}

/// A utility trait, that can be used to implement [`TryIntoDb`]. The error type
/// will be automatically mapped to [`crate::Error`].
pub trait TryIntoDbImp: DbRepresentable {
    type Err: std::error::Error + Send + Sync + 'static;

    fn try_into_db_imp(self) -> Result<Self::DbRepr, Self::Err>;
}

/// A utility trait, that can be used to implement [`TryIntoDb`]. The error type
/// will be automatically mapped to [`crate::Error`].
pub trait TryFromDbImp: DbRepresentable {
    type Err: std::error::Error + Send + Sync + 'static;

    fn try_from_db_imp(db_val: Self::DbRepr) -> Result<Self, Self::Err>;
}
