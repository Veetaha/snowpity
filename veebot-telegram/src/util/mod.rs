//! Assorted utility functions (missing batteries).
mod sqlx_ext;
mod teloxide_ext;

pub(crate) use sqlx_ext::*;
pub(crate) use teloxide_ext::*;

pub(crate) mod prelude {
    pub(crate) use super::sqlx_ext::ErrorExt;
    pub(crate) use super::sqlx_ext::FromDb;
    pub(crate) use super::sqlx_ext::IntoApp;
    pub(crate) use super::sqlx_ext::IntoDb;
    pub(crate) use super::sqlx_ext::TryIntoDb;
    pub(crate) use super::teloxide_ext::MessageKindExt;
    pub(crate) use super::teloxide_ext::SendMessageSettersExt;
}

use crate::{HttpError, UserError};
use async_trait::async_trait;
use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::fmt;
use std::str::FromStr;
use tracing::{debug, warn};

pub(crate) type DynError = dyn std::error::Error + Send + Sync;

macro_rules! def_url_base {
    ($ident:ident, $url:literal) => {
        fn $ident<T: AsRef<str>>(segments: impl IntoIterator<Item = T>) -> ::url::Url {
            let mut url: ::url::Url = $url.parse().unwrap();
            url.path_segments_mut().unwrap().extend(segments);
            url
        }
    };
}

pub(crate) use def_url_base;

#[async_trait]
pub(crate) trait ReqwestBuilderExt {
    async fn read_json<T: DeserializeOwned>(
        self,
        // url: Url,
        // query: &[(&str, &str)],
    ) -> crate::Result<T>;

    async fn read_bytes(self) -> crate::Result<Bytes>;
}

#[async_trait]
impl ReqwestBuilderExt for reqwest::RequestBuilder {
    async fn read_json<T: DeserializeOwned>(self) -> crate::Result<T> {
        let bytes = self.read_bytes().await?;

        serde_json::from_slice(&bytes).map_err(|err| {
            match std::str::from_utf8(&bytes) {
                Ok(response_body) => warn!(response_body, "Bad JSON response"),
                Err(utf8_decode_err) => warn!(
                    response_body = ?bytes,
                    ?utf8_decode_err,
                    "Bad JSON response"
                ),
            };
            crate::err_val!(HttpError::UnexpectedResponseJsonShape { source: err })
        })
    }

    async fn read_bytes(self) -> crate::error::Result<Bytes> {
        debug!(request = ?self, "sending HTTP request");

        let res = self
            // XXX: important for derpibooru (otherwise it responds with an html capcha page)
            .header("User-Agent", "Veebot")
            .send()
            .await
            .map_err(crate::err_ctx!(HttpError::SendRequest))?;

        let status = res.status();

        if status.is_client_error() || status.is_server_error() {
            let body = match res.text().await {
                Ok(it) => it,
                Err(err) => format!("Could not collect the error response body text: {}", err),
            };

            return Err(crate::err_val!(HttpError::BadResponseStatusCode {
                status,
                body
            }));
        }

        res.bytes()
            .await
            .map_err(crate::err_ctx!(HttpError::ReadResponse))
    }
}

pub(crate) fn create_http_client() -> reqwest::Client {
    teloxide::net::client_from_env()
}

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

#[must_use]
pub fn tracing_err<'a, E: std::error::Error + 'static>(
    err: &'a E,
) -> impl tracing::Value + std::fmt::Debug + 'a {
    err as &dyn std::error::Error
}
