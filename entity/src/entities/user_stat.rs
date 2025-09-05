//! i have to write this manually :(

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_stat")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    #[sea_orm(column_type = "Integer")]
    pub snipe: i64,
    #[sea_orm(column_type = "Integer")]
    pub sniped: i64,
    #[sea_orm(column_type = "Double", nullable)]
    pub snipe_rate: Option<f64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
