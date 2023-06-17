use teloxide::types::{ChatId, UserId};

crate::impl_try_into_db_via_newtype!(ChatId(i64));
crate::impl_try_into_db_via_newtype!(UserId(u64));
