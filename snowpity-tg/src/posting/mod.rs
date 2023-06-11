mod all_platforms;
mod error;
mod model;
mod service;
mod tg_upload;

pub(crate) mod platform;

pub(crate) mod derpibooru;
pub(crate) mod deviant_art;
pub(crate) mod twitter;

pub(crate) use all_platforms::*;
pub(crate) use error::*;
pub(crate) use model::*;
pub(crate) use service::*;
