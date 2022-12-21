use crate::prelude::*;
use crate::util::DynError;
use std::backtrace::Backtrace;
use std::fmt;
use thiserror::Error;
use tracing_error::SpanTrace;

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// Macro to reduce the boilerplate of creating crate-level errors.
/// It directly accepts the body of [`ErrorKind`] variant without type name qualification.
/// It also automatically calls [`Into`] conversion for each passed field.
macro_rules! err_val {
    (@val $variant_ident:ident $field_val:expr) => ($field_val);
    (@val $variant_ident:ident) => ($variant_ident);
    ($variant_path:path $({
        $( $field_ident:ident $(: $field_val:expr)? ),*
        $(,)?
    })?) => {{
        use $variant_path as Variant;

        $crate::error::Error::from(
            Variant $({$(
                $field_ident: ::std::convert::Into::into(
                    $crate::error::err_val!(@val $field_ident $($field_val)?)
                )
            ),*})?
        )
    }};
}

/// Shortcut for defining `map_err` closures that automatically forwards `source`
/// error to the variant.
macro_rules! err_ctx {
    ($variant_path:path $({ $($variant_fields:tt)* })?) => {
        |source| $crate::error::err_val!($variant_path { source, $($($variant_fields)*)? })
    };
}

use crate::derpi;
pub(crate) use err_ctx;
pub(crate) use err_val;
use std::sync::Arc;
use teloxide::types::MediaKind;

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
    User {
        #[from]
        source: UserError,
    },

    #[error(transparent)]
    HttpClient {
        #[from]
        source: HttpClientError,
    },

    #[error(transparent)]
    FtAi {
        #[from]
        source: FtAiError,
    },

    #[error(transparent)]
    Tg {
        #[from]
        source: teloxide::RequestError,
    },

    #[error(transparent)]
    Db { source: DbError },

    #[error(transparent)]
    Deserialize {
        #[from]
        source: DeserializeError,
    },

    // FIXME: display chain using human-readable format
    #[error("Multiple errors occurred: {errs:#?}")]
    Multiple { errs: Vec<Error> },

    #[error(transparent)]
    Media {
        #[from]
        source: MediaError,
    },

    #[error(transparent)]
    Io {
        #[from]
        source: IoError,
    },

    #[error("Not implemented yet: {message}")]
    Todo { message: &'static str },
}

impl<T: Into<DbError>> From<T> for ErrorKind {
    fn from(err: T) -> Self {
        Self::Db { source: err.into() }
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(err: std::io::Error) -> Self {
        Self::Io { source: err.into() }
    }
}

#[derive(Debug, Error)]
pub(crate) enum FtAiError {
    #[error("15.ai returned zero WAV files in the response")]
    MissingWavFile,

    #[error(
        "Failed to create a WAV reader, that is probably a bug, it must be infallible: {message}"
    )]
    CreateWavReader { message: &'static str },

    #[error("Failed to read WAV header returned by 15.ai: {message}")]
    ReadWavHeader { message: &'static str },

    #[error("Failed to read WAV samples returned by 15.ai: {message}")]
    ReadWavSamples { message: &'static str },

    #[error("Failed to encode the resampled WAV to OGG")]
    EncodeWavToOpus { source: ogg_opus::Error },

    #[error("Не правильный ввод. Проверьте имя персонажа на сайте 15.ai, или правильность введеного текста")]
    Service { source: Box<Error> },
}

/// Errors caused by interaction with the user.
/// These are most likely caused by humanz sending wrong input.
#[derive(Debug, Error)]
pub(crate) enum UserError {
    #[error("The specified image tags contain a comma (which is prohibited): {input}")]
    CommaInImageTag { input: String },

    // #[error("Запрет на слово уже существует (слово: {word})")]
    // BannedWordAlreadyExists { word: banned_words::Word },

    // #[error("Запрета на слово не существует (слово: {word})")]
    // BannedWordNotFound { word: banned_words::Word },

    // #[error("Чат уже существует в базе (chat_id: {chat_id})")]
    // ChatAlreadyExists { chat_id: ChatId },

    // #[error("Чат не был найден в базе (chat_id: {chat_id})")]
    // ChatNotFound { chat_id: ChatId },
    #[error("Текст для 15.ai не должен содержать цифр вне ARPAbet нотации")]
    FtaiTextContainsNumber,

    #[error(
        "Текст для 15.ai должен быть не более {} символов. Длина заданого текста: {actual_len}",
        crate::ftai::MAX_TEXT_LENGTH
    )]
    FtaiTextTooLong { actual_len: usize },

    #[error("Команда для 15.ai должна иметь название персонажа и текст через запятую: <персонаж>,<текст>")]
    FtaiInvalidFormat,

    #[error("No reply message in describe command")]
    NoReplyMessageInDescribe,
}

/// Errors at the layer of the HTTP API
#[derive(Debug, Error)]
pub(crate) enum HttpClientError {
    #[error("Failed to send an http request")]
    SendRequest { source: reqwest_middleware::Error },

    #[error("Failed to read http response")]
    ReadResponse { source: reqwest_middleware::Error },

    #[error("HTTP request has failed (http status code: {status}):\n{body}")]
    BadResponseStatusCode {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Received an unexpected response JSON object")]
    UnexpectedResponseJsonShape { source: serde_json::Error },
    // #[error("Failed to write bytes to a file")]
    // WriteToFile { source: std::io::Error },

    // #[error("Failed to flush bytes to a file")]
    // FlushToFile { source: std::io::Error },
}

/// Most likely unrecoverable errors from database communication layer
#[derive(Debug, Error)]
pub(crate) enum DbError {
    #[error("Failed to connect to the database")]
    Connect { source: sea_orm::DbErr },

    #[error("Failed to migrate the database")]
    Migrate { source: sea_orm::DbErr },

    #[error("Database query failed")]
    Query {
        #[from]
        source: sea_orm::DbErr,
    },

    #[error(
        "Failed to serialize app value into db repr.\n\
        App type: {app_ty}\n\
        Db type: {db_ty}\n\
        App value: {app_val:#?}"
    )]
    Serialize {
        source: Box<DynError>,
        app_ty: &'static str,
        db_ty: &'static str,
        app_val: Box<dyn fmt::Debug + Send + Sync>,
    },
    #[error(
        "Failed to deserialize db value into app repr.\n\
        App type: {app_ty}\n\
        Db type: {db_ty}\n\
        Db value: {db_val:#?}"
    )]
    Deserialize {
        source: Box<DynError>,
        app_ty: &'static str,
        db_ty: &'static str,
        db_val: Box<dyn fmt::Debug + Send + Sync>,
    },
}

#[derive(Debug, Error)]
pub(crate) enum DeserializeError {
    #[error("Failed to parse JSON as `{target_ty}`, input surrounded by backticks:\n```\n{input:?}\n```")]
    Json {
        target_ty: &'static str,
        input: String,
        source: serde_json::Error,
    },

    #[error(
        "Failed to decode the input as base64, input surrounded by backticks:\n```\n{input:?}\n```"
    )]
    Base64 {
        input: String,
        source: base64::DecodeError,
    },

    #[error(
        "The input is not a valid UTF8 sequence, input in base64: {}",
        base64::encode(input)
    )]
    Utf8 {
        input: Vec<u8>,
        source: std::str::Utf8Error,
    },
}

#[derive(Debug, Error)]
pub(crate) enum MediaError {
    #[error("Unexpected media kind for mime type {expected:?}: {media:#?}")]
    UnexpectedMediaKind {
        media: Box<MediaKind>,
        expected: derpi::MimeType,
    },

    #[error(
        "The size of the requested file `{actual}` bytes \
        exceeds the limit of `{max}` byttes"
    )]
    FileTooBig { actual: u64, max: u64 },

    #[error("Failed to spawn `ffmpeg` process")]
    SpawnFfmpeg { source: std::io::Error },

    #[error("Failed to wait for `ffmpeg` process")]
    WaitForFfmpeg { source: std::io::Error },

    #[error("`ffmpeg` failed with the exit code {status}")]
    Ffmpeg { status: std::process::ExitStatus },
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

    pub(crate) fn is_user_error(&self) -> bool {
        matches!(self.imp.kind, ErrorKind::User { .. })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error (id: {}): {}", self.imp.id, self.imp.kind)?;

        if let Some(backtrace) = &self.imp.backtrace {
            write!(f, "\n{:?}", backtrace)?;
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
