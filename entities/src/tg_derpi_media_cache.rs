use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tg_derpi_media_cache")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "BigInteger")]
    pub derpi_id: i64,
    pub tg_file_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
