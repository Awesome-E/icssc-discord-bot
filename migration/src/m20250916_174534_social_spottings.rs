use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Message::Table)
                    .add_column(boolean(Message::IsSocial).default(false))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Message::Table)
                    .modify_column(boolean(Message::IsSocial))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // hope and pray there's no social data, because they'll all be converted to snipes

        manager
            .alter_table(
                Table::alter()
                    .table(Message::Table)
                    .drop_column(Message::IsSocial)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Message {
    Table,
    IsSocial,
}
