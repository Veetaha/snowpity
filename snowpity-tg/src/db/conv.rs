use crate::util::DynError;
use crate::{err_ctx, DbError, Result};
use std::fmt;

pub(crate) fn try_into_db<App, Db>(app_val: App) -> Result<Db>
where
    App: TryInto<Db> + Clone + fmt::Debug + Send + Sync + 'static,
    App::Error: Into<Box<DynError>>,
{
    app_val
        .clone()
        .try_into()
        .map_err(err_ctx!(DbError::Serialize {
            app_ty: std::any::type_name::<App>(),
            db_ty: std::any::type_name::<Db>(),
            app_val: Box::new(app_val) as Box<_>
        }))
}

// pub(crate) fn _try_from_db<App, Db>(db_val: Db) -> Result<App>
// where
//     Db: TryInto<App> + Clone + fmt::Debug + Send + Sync + 'static,
//     Db::Error: Into<Box<DynError>>,
// {
//     db_val
//         .clone()
//         .try_into()
//         .map_err(err_ctx!(DbError::Deserialize {
//             app_ty: std::any::type_name::<App>(),
//             db_ty: std::any::type_name::<Db>(),
//             db_val: Box::new(db_val) as Box<_>
//         }))
// }
