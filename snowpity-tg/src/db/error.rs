/// Most likely unrecoverable errors from database communication layer
#[derive(Debug, thiserror::Error)]
pub(crate) enum DbError {
    #[error("Failed to connect to the database")]
    Connect { source: sqlx::Error },

    #[error("Failed to migrate the database")]
    Migrate { source: sqlx::Error },

    #[error("Database query failed")]
    Query {
        #[from]
        source: sqlx::Error,
    },

    #[error(transparent)]
    SqlxBat {
        #[from]
        source: sqlx_bat::Error,
    },
}

impl From<sqlx::Error> for crate::ErrorKind {
    fn from(err: sqlx::Error) -> Self {
        Self::Db { source: err.into() }
    }
}

impl From<sqlx_bat::Error> for crate::ErrorKind {
    fn from(err: sqlx_bat::Error) -> Self {
        Self::Db { source: err.into() }
    }
}
