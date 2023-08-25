//! Assorted utility functions (missing batteries).
mod chrono_ext;
mod std_ext;
mod teloxide_ext;

pub mod display;
pub mod encoding;
pub mod media_conv;
pub mod process;
pub mod retry;
pub mod temp_file;
pub mod tokio;
pub mod units;
pub mod url;

pub(crate) use std_ext::*;

pub(crate) mod prelude {
    pub(crate) use super::chrono_ext::DateTimeExt;
    pub(crate) use super::std_ext::prelude::*;
    pub(crate) use super::teloxide_ext::prelude::*;
}

pub(crate) type DynResult<T = (), E = Box<DynError>> = std::result::Result<T, E>;
pub(crate) type DynError = dyn std::error::Error + Send + Sync;
