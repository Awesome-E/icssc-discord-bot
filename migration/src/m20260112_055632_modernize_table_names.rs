use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .rename_table(
                TableRenameStatement::new()
                    .table(Message::Table, SpottingMessage::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .rename_table(
                TableRenameStatement::new()
                    .table(Snipe::Table, SpottingVictim::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .rename_table(
                TableRenameStatement::new()
                    .table(OptOut::Table, SnipeOptOut::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .rename_table(
                TableRenameStatement::new()
                    .table(SnipeOptOut::Table, OptOut::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .rename_table(
                TableRenameStatement::new()
                    .table(SpottingVictim::Table, Snipe::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .rename_table(
                TableRenameStatement::new()
                    .table(SpottingMessage::Table, Message::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Message {
    Table,
}

#[derive(DeriveIden)]
enum Snipe {
    Table,
}

#[derive(DeriveIden)]
enum OptOut {
    Table,
}

#[derive(DeriveIden)]
enum SpottingMessage {
    Table,
}

#[derive(DeriveIden)]
enum SpottingVictim {
    Table,
}

#[derive(DeriveIden)]
enum SnipeOptOut {
    Table,
}
