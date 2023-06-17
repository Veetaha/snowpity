type DynError = dyn std::error::Error + Send + Sync + 'static;
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Unrecoverable errors from database communication layer
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database query failed")]
    Query {
        #[from]
        source: sqlx::Error,
    },

    #[error(
        "Failed to serialize app value into db repr.\n\
        App type: {app_ty}\n\
        Db type: {db_ty}\n\
        App value: {app_val}"
    )]
    Serialize {
        source: Box<DynError>,
        app_ty: &'static str,
        db_ty: &'static str,
        app_val: String,
    },

    #[error(
        "Failed to deserialize db value into app repr.\n\
        App type: {app_ty}\n\
        Db type: {db_ty}\n\
        Db value: {db_val}"
    )]
    Deserialize {
        source: Box<DynError>,
        app_ty: &'static str,
        db_ty: &'static str,
        db_val: String,
    },
}
