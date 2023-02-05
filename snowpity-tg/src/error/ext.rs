use super::{err, DynError, ErrorKind, Result};
use easy_ext::ext;

#[ext(ResultExt)]
impl<T, E> Result<T, E> {
    #[track_caller]
    pub fn fatal_ctx<S>(self, message: impl FnOnce() -> S) -> Result<T>
    where
        S: Into<String>,
        E: Into<Box<DynError>>,
    {
        // Not using closures (e.g. `map_err`), because `#[track_caller]`
        // doesn't propagate to them.
        match self {
            Ok(value) => Ok(value),
            Err(err) => err!(ErrorKind::Fatal {
                message: message().into(),
                source: Some(err.into()),
            }),
        }
    }
}

#[ext(OptionExt)]
impl<T> Option<T> {
    #[track_caller]
    pub fn fatal_ctx<S>(self, message: impl FnOnce() -> S) -> Result<T>
    where
        S: Into<String>,
    {
        // Not using closures (e.g. `ok_or_else`), because `#[track_caller]`
        // doesn't propagate to them.
        match self {
            Some(value) => Ok(value),
            None => err!(ErrorKind::Fatal {
                message: message().into(),
                source: None,
            }),
        }
    }
}
