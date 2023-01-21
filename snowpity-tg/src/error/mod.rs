mod macros;

use crate::prelude::*;
use crate::util::DynError;
use std::backtrace::Backtrace;
use std::fmt;
use std::sync::Arc;
use thiserror::Error;
use tracing_error::SpanTrace;

pub(crate) use macros::*;

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// Describes any possible error that may happen in the application lifetime.
#[derive(Clone)]
pub struct Error {
    imp: Arc<ErrorImp>,
}

struct ErrorImp {
    /// Small identifier used for debugging purposes.
    /// It is mentioned in the chat when the error happens.
    /// This way we as developers can copy it and lookup the logs using this id.
    pub(crate) id: String,
    backtrace: Option<Backtrace>,
    kind: ErrorKind,

    // Participates only in debug impl
    #[allow(dead_code)]
    pub(crate) spantrace: SpanTrace,
}

#[derive(Error, Debug)]
pub(crate) enum ErrorKind {
    #[error(transparent)]
    FtaiCommand {
        #[from]
        source: crate::tg::FtaiCommandError,
    },

    #[error(transparent)]
    DescribeCommand {
        #[from]
        source: crate::tg::DescribeCommandError,
    },

    #[error(transparent)]
    HttpClient {
        #[from]
        source: crate::http::HttpClientError,
    },

    #[error(transparent)]
    FtAi {
        #[from]
        source: crate::ftai::FtAiError,
    },

    #[error(transparent)]
    Tg {
        #[from]
        source: teloxide::RequestError,
    },

    #[error(transparent)]
    Db {
        #[from]
        source: crate::db::DbError,
    },

    #[error(transparent)]
    Deserialize {
        #[from]
        source: crate::encoding::DeserializeError,
    },

    // FIXME: display chain using human-readable format
    #[error("Multiple errors occurred: {errs:#?}")]
    Multiple { errs: Vec<Error> },

    #[error(transparent)]
    MediaCache {
        #[from]
        source: crate::tg::MediaCacheError,
    },

    #[error(transparent)]
    Twitter {
        #[from]
        source: crate::posting::twitter::TwitterError,
    },

    #[error(transparent)]
    MediaConv {
        #[from]
        source: crate::media_conv::MediaConvError,
    },

    #[error(transparent)]
    Io {
        #[from]
        source: IoError,
    },

    #[error("Not implemented yet: {message}")]
    // This variant is used only as a gag when we postpone the implementation
    // for the future, therefore it's not always used.
    #[allow(dead_code)]
    Todo { message: &'static str },

    /// Unrecoverable kind of error, that is not supposed to happen, but when
    /// it happens we can't do anything reasonable about it, so no structural
    /// error handling is possible, this error is just propagated to the top.
    #[error("FATAL: {message}")]
    Fatal {
        message: String,
        source: Option<Box<DynError>>,
    },
}

impl From<std::io::Error> for ErrorKind {
    fn from(err: std::io::Error) -> Self {
        Self::Io { source: err.into() }
    }
}

#[derive(Debug, Error)]
pub(crate) enum IoError {
    #[error("Failed to create a temporary file")]
    CreateTempFile { source: std::io::Error },

    #[error(transparent)]
    Other {
        #[from]
        source: std::io::Error,
    },
}

impl Error {
    pub(crate) fn id(&self) -> &str {
        &self.imp.id
    }

    /// Errors caused by interaction with the user.
    /// These are most likely caused by humanz sending wrong input.
    pub(crate) fn is_user_error(&self) -> bool {
        match &self.imp.kind {
            ErrorKind::FtaiCommand { .. } | ErrorKind::DescribeCommand { .. } => true,
            ErrorKind::Multiple { errs } => errs.iter().all(Self::is_user_error),
            ErrorKind::HttpClient { .. }
            | ErrorKind::Twitter { .. }
            | ErrorKind::FtAi { .. }
            | ErrorKind::Tg { .. }
            | ErrorKind::Db { .. }
            | ErrorKind::Deserialize { .. }
            | ErrorKind::MediaCache { .. }
            | ErrorKind::MediaConv { .. }
            | ErrorKind::Io { .. }
            | ErrorKind::Todo { .. }
            | ErrorKind::Fatal { .. } => false,
        }
    }

    pub(crate) fn kind(&self) -> &ErrorKind {
        &self.imp.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error (id: {}): {}", self.imp.id, self.imp.kind)?;

        if let Some(backtrace) = &self.imp.backtrace {
            write!(f, "\n{backtrace:?}")?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.imp.kind.source()
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)?;
        fmt::Display::fmt(&self.imp.spantrace, f)
    }
}

impl<T: Into<ErrorKind>> From<T> for Error {
    #[track_caller]
    fn from(kind: T) -> Self {
        let kind: ErrorKind = kind.into();
        // No need for a backtrace if the error is an expected one
        // TODO: add ability to send multiple message to overcome message limit
        // or truncate the backtrace
        // let backtrace = if !kind.is_user_error() {
        //     // We don't use `bool::then` adapter to reduce the backtrace
        //     None
        //     // Some(Backtrace::force_capture())
        // } else {
        //     None
        // };

        let imp = ErrorImp {
            kind,
            id: nanoid::nanoid!(6),
            backtrace: None,
            spantrace: SpanTrace::capture(),
        };

        let err = Self { imp: Arc::new(imp) };

        trace!(err = tracing_err(&err), "Created an error");

        err
    }
}
