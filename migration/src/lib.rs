pub use sea_orm_migration::prelude::*;

mod m20221201_213436_create_tg_derpi_media_cache_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(
            m20221201_213436_create_tg_derpi_media_cache_table::Migration,
        )]
    }
}
