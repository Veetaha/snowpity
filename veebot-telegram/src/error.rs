use std::fmt;

use thiserror::Error;
use backtrace::Backtrace;
use tracing::trace;
use crate::util::tracing_err;

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// Macro to reduce the boilerplate of creating crate-level errors.
/// It directly accepts the body of [`ErrorKind`] variant without type name qualification.
/// It also automatically calls [`Into`] conversion for each passed field.
macro_rules! err_val {
    (@val $variant_ident:ident $field_val:expr) => ($field_val);
    (@val $variant_ident:ident) => ($variant_ident);
    ($variant_ident:ident $({
        $( $field_ident:ident $(: $field_val:expr)? ),*
        $(,)?
    })?) => {
        $crate::error::Error::from(
            $crate::error::ErrorKind::$variant_ident $({$(
                $field_ident: ::std::convert::Into::into(
                    $crate::error::err_val!(@val $field_ident $($field_val)?)
                )
            ),*})?
        )
    };
}

/// Shortcut for defining `map_err` closures that automatically forwards `source`
/// error to the variant.
macro_rules! err_ctx {
    ($variant_ident:ident $({ $($variant_fields:tt)* })?) => {
        |source| $crate::error::err_val!($variant_ident { source, $($($variant_fields)*)? })
    };
}

pub(crate) use err_ctx;
pub(crate) use err_val;

#[derive(Debug)]
pub struct Error {
    /// Small identifier used for debugging purposes.
    /// It is mentioned in the chat when the error happens.
    /// This way we as developers can copy it and lookup the logs using this id.
    pub(crate) id: String,
    pub(crate) backtrace: Option<Backtrace>,
    pub(crate) kind: ErrorKind,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(error_id: {}) {}", self.id, self.kind)?;

        if let Some(backtrace) = &self.backtrace {
            write!(f, "\n{:?}", backtrace)?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<T: Into<ErrorKind>> From<T> for Error {
    #[track_caller]
    fn from(kind: T) -> Self {
        let kind: ErrorKind = kind.into();
        // No need for a backtrace if the error is an expected one
        let backtrace = if kind.is_user_error() {
            // We don't use `bool::then` adapter to reduce the backtrace
            Some(Backtrace::new())
        } else {
            None
        };

        let err = Self {
            kind,
            id: nanoid::nanoid!(6),
            backtrace,
        };

        trace!(err = tracing_err(&err), "Created an error");

        err
    }
}

impl ErrorKind {
    fn is_user_error(&self) -> bool {
        match self {
            ErrorKind::TgSend { .. }
            | ErrorKind::SendHttpRequest { .. }
            | ErrorKind::ReadHttpResponse { .. }
            | ErrorKind::BadHttpResponseStatusCode { .. }
            | ErrorKind::UnexpectedHttpResponseJsonShape { .. } => false,
            ErrorKind::CommaInImageTag { .. } => true,
        }
    }
}

#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("The specified image tags contain a comma (which is prohibited): {input}")]
    CommaInImageTag { input: String },

    #[error("Failed to send an http request")]
    SendHttpRequest { source: reqwest::Error },

    #[error("Failed to read http response")]
    ReadHttpResponse { source: reqwest::Error },

    #[error("HTTP request has failed (http status code: {status}):\n{body}")]
    BadHttpResponseStatusCode {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Received an unexpected response JSON object")]
    UnexpectedHttpResponseJsonShape { source: serde_json::Error },

    #[error("Request to Telegram failed")]
    TgSend {
        #[from]
        #[source]
        source: teloxide::RequestError,
    },
}
