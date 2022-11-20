use crate::util::DynError;
use crate::{Result, derpi};
use duplicate::duplicate;
use sqlx::postgres::types::PgInterval;
use std::time::Duration;
use teloxide::types::{ChatId, UserId};

use super::{DbRepresentable, IntoDb, TryFromDbImp, TryIntoDbImp};

// impl DbRepresentable for censy::TemplatePhrase {
//     type DbRepr = String;
// }

// impl IntoDb for censy::TemplatePhrase {
//     fn into_db(self) -> Self::DbRepr {
//         self.into_string()
//     }
// }

// impl TryFromDbImp for censy::TemplatePhrase {
//     type Err = censy::TemplatePhraseError;

//     fn try_from_db_imp(db_val: Self::DbRepr) -> Result<Self, Self::Err> {
//         Self::new(&db_val)
//     }
// }

// duplicate! {
//     [
//         Ty       DbReprTy;
//         [ChatId] [i64];
//         [UserId] [u64];
//     ]

//     impl DbRepresentable for Ty {
//         type DbRepr = DbReprTy;
//     }

//     impl TryIntoDbImp for Ty {
//         fn try_into_db_imp(self) -> Self::DbRepr {
//             self.0.try_into_db_imp()
//         }
//     }

//     impl TryFromDbImp for Ty {
//         type Err = std::num::ParseIntError;

//         fn try_from_db_imp(db_val: Self::DbRepr) -> Result<Self, Self::Err> {
//             db_val.parse().map(Self)
//         }
//     }
// }

// Recursive expansion of duplicate! macro
// ========================================

// impl DbRepresentable for ChatId {
//     type DbRepr = i64;
// }
// impl TryIntoDbImp for ChatId {
//     fn try_into_db_imp(self) -> Self::DbRepr {
//         self.0.try_into_db_imp()
//     }
// }
// impl TryFromDbImp for ChatId {
//     type Err = std::num::ParseIntError;
//     fn try_from_db_imp(db_val: Self::DbRepr) -> Result<Self, Self::Err> {
//         db_val.parse().map(Self)
//     }
// }
impl DbRepresentable for UserId {
    type DbRepr = u64;
}
// impl TryIntoDbImp for UserId {
//     fn try_into_db_imp(self) -> Self::DbRepr {
//         self.0.try_into_db_imp()
//     }
// }
// impl TryFromDbImp for UserId {
//     type Err = std::num::ParseIntError;
//     fn try_from_db_imp(db_val: Self::DbRepr) -> Result<Self, Self::Err> {
//         db_val.parse().map(Self)
//     }
// }

#[test]
fn sandbox_foo() {
    let user_id = UserId(32);
    let db = user_id.try_into_db_imp();

}

impl DbRepresentable for u64 {
    type DbRepr = i64;
}
