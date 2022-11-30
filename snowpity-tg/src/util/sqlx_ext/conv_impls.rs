use crate::util::DynError;
use crate::Result;
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

duplicate! {
    [Ty; [ChatId]; [UserId]]

    impl DbRepresentable for Ty {
        type DbRepr = String;
    }

    impl IntoDb for Ty {
        fn into_db(self) -> Self::DbRepr {
            self.to_string()
        }
    }

    impl TryFromDbImp for Ty {
        type Err = std::num::ParseIntError;

        fn try_from_db_imp(db_val: Self::DbRepr) -> Result<Self, Self::Err> {
            db_val.parse().map(Self)
        }
    }
}
