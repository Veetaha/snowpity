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
    TG_DERPI_MEDIA_CACHE_MEDIA_ID_PK = "tg_derpi_media_cache_media_id_pk";
    // TG_CHATS_PK = "tg_chats_pk";
    // TG_CHAT_AND_BANNED_WORD_COMPOSITE_PK = "tg_chat_and_banned_word_composite_pk";
    // TG_CHATS_FK = "tg_chats_fk";
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
            .filter(|&&constraint| !actual_constraints.contains(constraint))
            .collect();

        assert_eq!(
            non_existing_constraints,
            &[] as &[&&str],
            "Some constraints were not defined in migrations. Actual constraints: {actual_constraints:?}",
        );
    }

    async fn fetch_all(&self) -> Result<HashSet<String>> {
        let query = sqlx::query!(r#"
            select conname as constraint
            from pg_catalog.pg_constraint con
            inner join pg_catalog.pg_class rel on rel.oid = con.conrelid
            inner join pg_catalog.pg_namespace nsp on nsp.oid = connamespace
            where nsp.nspname = 'public'
        "#);

        query
            .fetch(&self.pool)
            .map_ok(|record| record.constraint)
            .try_collect()
            .err_into()
            .await
    }
}
