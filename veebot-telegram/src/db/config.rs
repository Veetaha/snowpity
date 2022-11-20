use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) url: url::Url,

    #[serde(default = "default_database_pool_size")]
    pub(crate) pool_size: u32,
}

fn default_database_pool_size() -> u32 {
    // Postgres instance has 100 connections limit.
    // However, we also reserve 2 connections for ad-hoc db administration purposes
    // via pg_admin, for example.
    98
}
