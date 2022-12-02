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
