mod conv;
mod error;
mod misc;
mod sea_query_ext;

pub use conv::*;
pub use error::*;
pub use misc::*;
pub use sea_query_ext::*;

pub mod prelude {
    pub use crate::{
        DbRepresentable as _, ErrorExt as _, IntoDb as _, SqlxBinderExt as _, TryFromDb as _,
        TryIntoApp as _, TryIntoDb as _,
    };
}

#[doc(hidden)]
pub mod imp {
    pub use sea_query;
}
