use crate::{err_val, DbError, Result};
use std::{any, fmt};

/// A type that has a database representation specified in [`DbRepresentable::DbRepr`]
pub(crate) trait DbRepresentable:
    fmt::Debug + Clone + Sized + Send + Sync + 'static
{
    type DbRepr: fmt::Debug + Clone + Sized + Send + Sync + 'static;
}

/// A type that can't be losslessly converted to database repr.
/// Mostly convenient only for mapping the error type to crate's [`DbError`].
pub(crate) trait TryIntoDb: DbRepresentable {
    fn try_into_db(self) -> Result<Self::DbRepr>;
}

/// A type that can be losslessly converted to database repr.
pub(crate) trait IntoDb: DbRepresentable {
    fn into_db(self) -> Self::DbRepr;
}

pub(crate) trait TryFromDb: DbRepresentable {
    fn try_from_db(val: Self::DbRepr) -> Result<Self>;
}

pub(crate) trait TryIntoApp<A> {
    fn try_into_app(self) -> Result<A>;
}

pub(crate) trait TryIntoDbImp: DbRepresentable {
    type Err: std::error::Error + Send + Sync + 'static;

    fn try_into_db_imp(self) -> Result<Self::DbRepr, Self::Err>;
}

pub(crate) trait TryFromDbImp: DbRepresentable {
    type Err: std::error::Error + Send + Sync + 'static;

    fn try_from_db_imp(db_val: Self::DbRepr) -> Result<Self, Self::Err>;
}

impl<A: TryFromDb> TryIntoApp<A> for A::DbRepr {
    fn try_into_app(self) -> Result<A> {
        A::try_from_db(self)
    }
}

impl<A: TryIntoDbImp> TryIntoDb for A {
    fn try_into_db(self) -> Result<Self::DbRepr> {
        self.clone().try_into_db_imp().map_err(|source| {
            err_val!(DbError::Serialize {
                source,
                app_ty: any::type_name::<Self>(),
                db_ty: any::type_name::<Self::DbRepr>(),
                app_val: Box::new(self) as Box<_>,
            })
        })
    }
}

impl<A: TryFromDbImp> TryFromDb for A {
    fn try_from_db(db_val: Self::DbRepr) -> Result<Self> {
        Self::try_from_db_imp(db_val.clone()).map_err(|source| {
            err_val!(DbError::Deserialize {
                source,
                app_ty: any::type_name::<Self>(),
                db_ty: any::type_name::<Self::DbRepr>(),
                db_val: Box::new(db_val) as Box<_>,
            })
        })
    }
}

// macro_rules! impl_try_into_db_via_try_into {
//     ($ty:ty) => {
//         impl TryIntoDbImp for A {
//             type Err = <A as TryInto<A::DbRepr>>::Error;

//             fn try_into_db_imp(self) -> Result<Self::DbRepr, Self::Err> {
//                 self.try_into()
//             }
//         }
//     };
// }


impl<A> TryIntoDbImp for A
where
    A: DbRepresentable,
    A::DbRepr: TryFrom<A>,
    <A::DbRepr as TryFrom<A>>::Error: std::error::Error + Send + Sync + 'static,
{
    type Err = <A as TryInto<A::DbRepr>>::Error;

    fn try_into_db_imp(self) -> Result<Self::DbRepr, Self::Err> {
        self.try_into()
    }
}


// impl<A: TryFrom<A::DbRepr> + DbRepresentable> TryFromDbImp for A
// where
//     <A as TryFrom<A::DbRepr>>::Error: std::error::Error + Send + Sync + 'static,
// {
//     type Err = <A as TryFrom<A::DbRepr>>::Error;

//     fn try_from_db_imp(db_val: Self::DbRepr) -> Result<Self, Self::Err> {
//         Self::try_from(db_val)
//     }
// }

impl<T: DbRepresentable> DbRepresentable for Option<T> {
    type DbRepr = Option<T::DbRepr>;
}

impl<T: TryFromDb> TryFromDb for Option<T> {
    fn try_from_db(val: Self::DbRepr) -> Result<Self> {
        val.map(<_>::try_from_db).transpose()
    }
}
