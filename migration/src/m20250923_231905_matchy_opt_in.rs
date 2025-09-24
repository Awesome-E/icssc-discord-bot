use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MatchyMeetupOptIn::Table)
                    .if_not_exists()
                    .col(big_integer(MatchyMeetupOptIn::UserId))
                    .col(timestamp(MatchyMeetupOptIn::CreatedAt).default(Expr::cust("NOW()")))
                    .primary_key(Index::create().col(MatchyMeetupOptIn::UserId))
                    .to_owned()
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MatchyMeetupOptIn::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MatchyMeetupOptIn {
    Table,
    UserId,
    CreatedAt,
}
