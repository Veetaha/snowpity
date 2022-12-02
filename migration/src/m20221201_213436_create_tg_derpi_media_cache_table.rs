use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut stmt = Table::create();
        stmt.table(TgDerpiMediaCache::Table)
            .col(
                ColumnDef::new(TgDerpiMediaCache::DerpiId)
                    .big_integer()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(TgDerpiMediaCache::TgFileId)
                    .string_len(100)
                    .not_null(),
            );
        manager.create_table(stmt).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut stmt = Table::drop();
        stmt.table(TgDerpiMediaCache::Table);
        manager.drop_table(stmt).await
    }
}

#[derive(Iden)]
enum TgDerpiMediaCache {
    Table,
    DerpiId,
    TgFileId,
}
