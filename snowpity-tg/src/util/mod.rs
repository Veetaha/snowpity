//! Assorted utility functions (missing batteries).
mod chrono_ext;
mod std_ext;
mod teloxide_ext;
mod tokio_ext;

pub(crate) mod encoding;
pub(crate) mod http;

pub(crate) use chrono_ext::*;
pub(crate) use std_ext::*;
// pub(crate) use teloxide_ext::*;
pub(crate) use tokio_ext::*;

// We don't care if some of the imports here are not used. They may be used
// at some point. It's just convenient not to import them manually all the
// time a new logging macro is needed.
pub(crate) mod prelude {
    pub(crate) use super::chrono_ext::DateTimeExt as _;
    pub(crate) use super::http::RequestBuilderExt as _;
    pub(crate) use super::std_ext::prelude::*;
    pub(crate) use super::teloxide_ext::prelude::*;
}

use crate::{Result, UserError};
use std::fmt;
use std::str::FromStr;

pub(crate) type DynResult<T = (), E = Box<DynError>> = std::result::Result<T, E>;
pub(crate) type DynError = dyn std::error::Error + Send + Sync;

macro_rules! def_url_base {
    ($vis:vis $ident:ident, $url:literal) => {
        $vis fn $ident<T: AsRef<str>>(segments: impl IntoIterator<Item = T>) -> ::url::Url {
            let mut url: ::url::Url = $url.parse().unwrap();
            url.path_segments_mut().unwrap().extend(segments);
            url
        }
    };
}

pub(crate) use def_url_base;

// A string without commas
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) struct ThemeTag(String);

impl fmt::Display for ThemeTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for ThemeTag {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<ThemeTag, Self::Err> {
        let input = s.to_owned();
        if s.contains(',') {
            return Err(crate::err_val!(UserError::CommaInImageTag { input }));
        }
        Ok(ThemeTag(input))
    }
}
