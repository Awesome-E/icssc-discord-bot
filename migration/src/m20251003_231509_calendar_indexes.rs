use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_server_calendar_webhook_id")
                    .table(ServerCalendar::Table)
                    .col(ServerCalendar::WebhookId)
                    .index_type(IndexType::BTree)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ServerCalendar::Table)
                    .add_column(
                        ColumnDef::new(ServerCalendar::WebhookLastUpdated)
                            .date_time()
                            .to_owned()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(ServerCalendar::WebhookGCalResourceId)
                            .text()
                            .to_owned()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ServerCalendar::Table)
                    .drop_column(ServerCalendar::WebhookLastUpdated)
                    .drop_column(ServerCalendar::WebhookGCalResourceId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(Index::drop().name("idx_server_calendar_webhook_id").to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServerCalendar {
    Table,
    WebhookId,
    WebhookLastUpdated,
    WebhookGCalResourceId,
}
