mod formatting;
mod future_ext;
mod init;

pub use formatting::tracing_err;
pub use init::init_logging;

pub(crate) mod prelude {
    pub(crate) use super::formatting::{tracing_duration, tracing_err};
    pub(crate) use super::future_ext::FutureExt as _;
    pub(crate) use super::future_ext::TryFutureExt as _;

    // We don't care if some of the imports here are not used. They may be used
    // at some point. It's just convenient not to import them manually all the
    // time a new logging macro is needed.
    #[allow(unused_imports)]
    pub(crate) use tracing::{
        debug, debug_span, error, error_span, info, info_span, instrument, trace, trace_span, warn,
        warn_span, Instrument as _,
    };
}
