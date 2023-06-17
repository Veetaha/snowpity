use crate::{
    DbRepresentable, Error, Result, TryFromDb, TryFromDbImp, TryIntoApp, TryIntoDb, TryIntoDbImp,
};
use std::any::type_name;

impl<A: TryFromDb> TryIntoApp<A> for A::DbRepr {
    fn try_into_app(self) -> Result<A> {
        A::try_from_db(self)
    }
}

impl<A: TryIntoDbImp> TryIntoDb for A {
    fn try_into_db(self) -> Result<Self::DbRepr> {
        let app_val_dbg = format!("{self:#?}");
        self.try_into_db_imp().map_err(|source| Error::Serialize {
            source: Box::new(source),
            app_ty: type_name::<Self>(),
            db_ty: type_name::<Self::DbRepr>(),
            app_val: app_val_dbg,
        })
    }
}

impl<A: TryFromDbImp> TryFromDb for A {
    fn try_from_db(db_val: Self::DbRepr) -> Result<Self> {
        let db_val_dbg = format!("{db_val:#?}");
        Self::try_from_db_imp(db_val).map_err(|source| Error::Deserialize {
            source: Box::new(source),
            app_ty: type_name::<Self>(),
            db_ty: type_name::<Self::DbRepr>(),
            db_val: db_val_dbg,
        })
    }
}

impl<T: DbRepresentable> DbRepresentable for Option<T> {
    type DbRepr = Option<T::DbRepr>;
}

impl<T: TryFromDb> TryFromDb for Option<T> {
    fn try_from_db(val: Self::DbRepr) -> Result<Self> {
        val.map(<_>::try_from_db).transpose()
    }
}

#[macro_export]
macro_rules! impl_try_into_db_via_newtype {
    ($app_ident:ident($app_ty:ty)) => {
        impl $crate::DbRepresentable for $app_ident {
            type DbRepr = <$app_ty as $crate::DbRepresentable>::DbRepr;
        }

        impl $crate::TryIntoDb for $app_ident {
            fn try_into_db(self) -> $crate::Result<Self::DbRepr> {
                $crate::TryIntoDb::try_into_db(self.0)
            }
        }

        impl $crate::TryFromDb for $app_ident {
            fn try_from_db(db_val: Self::DbRepr) -> $crate::Result<Self> {
                $crate::TryFromDb::try_from_db(db_val).map($app_ident)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_try_into_from_db_via_std {
    ($app_ty:ty, $db_ty:ty) => {
        impl $crate::DbRepresentable for $app_ty {
            type DbRepr = $db_ty;
        }

        impl $crate::TryIntoDbImp for $app_ty {
            type Err = <Self as TryInto<Self::DbRepr>>::Error;

            fn try_into_db_imp(self) -> Result<Self::DbRepr, Self::Err> {
                self.try_into()
            }
        }

        impl $crate::TryFromDbImp for $app_ty {
            type Err = <Self as TryFrom<Self::DbRepr>>::Error;

            fn try_from_db_imp(db_val: Self::DbRepr) -> Result<Self, Self::Err> {
                Self::try_from(db_val)
            }
        }
    };
}
