use futures::prelude::*;
use std::collections::HashSet;

use crate::Result;

macro_rules! def_constraints {
    ($($ident:ident)*) => {
        $(
            // The variable name will have the same casing convention as the constraint name.
            #[allow(non_upper_case_globals)]
            pub(crate) const $ident: &str = stringify!($ident);
        )*
        const ALL_CONSTRAINTS: &[&str] = &[$($ident),*];
    }
}

def_constraints! {
    tg_chat_pk
    tg_derpi_media_cache_pk
    tg_twitter_media_cache_pk
}

pub(crate) async fn validate(pool: sqlx::PgPool) {
    Constraints::new(pool).validate().await
}

struct Constraints {
    pool: sqlx::PgPool,
}

impl Constraints {
    fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    async fn validate(&self) {
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
        let query = sqlx::query_scalar!(
            r#"
            select conname
            from pg_catalog.pg_constraint
            inner join pg_catalog.pg_namespace nsp
            on nsp.oid = connamespace and nsp.nspname = 'public'
            "#
        );

        query.fetch(&self.pool).try_collect().err_into().await
    }
}
