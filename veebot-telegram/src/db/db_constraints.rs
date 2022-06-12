use futures::prelude::*;
use std::collections::HashSet;

use crate::Result;

macro_rules! def_constraints {
    ($($ident:ident = $val:literal;)*) => {
        $( pub(crate) const $ident: &str = $val; )*
        const ALL_CONSTRAINTS: &[&str] = &[$($ident),*];
    }
}

def_constraints! {
    TG_CHATS_PK = "tg_chats_pk";
    TG_CHAT_AND_PATTERN_COMPOSITE_PK = "tg_chat_and_pattern_composite_pk";
    TG_CHATS_FK = "tg_chats_fk";
}

pub(crate) struct DbConstraints {
    pool: sqlx::PgPool,
}

impl DbConstraints {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub(crate) async fn validate(&self) {
        let actual_constraints = self
            .fetch_all()
            .await
            .expect("BUG: failed to fetch constraints for validation");

        let non_existing_constraints: Vec<_> = ALL_CONSTRAINTS
            .iter()
            .filter(|&&constraint| actual_constraints.contains(constraint))
            .collect();

        assert_eq!(
            non_existing_constraints,
            &[] as &[&&str],
            "Some constraints were not defined in migrations"
        );
    }

    async fn fetch_all(&self) -> Result<HashSet<String>> {
        let query = sqlx::query!(
            r#"
            SELECT conname as "constraint!"
            FROM pg_catalog.pg_constraint con
            INNER JOIN pg_catalog.pg_class rel ON rel.oid = con.conrelid
            INNER JOIN pg_catalog.pg_namespace nsp ON nsp.oid = connamespace
            WHERE nsp.nspname = 'public'
        "#
        );

        query
            .fetch(&self.pool)
            .map_ok(|record| record.constraint)
            .try_collect()
            .err_into()
            .await
    }
}
