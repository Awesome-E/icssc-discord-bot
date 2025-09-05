use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MatchyMeetupRound::Table)
                    .if_not_exists()
                    .col(pk_auto(MatchyMeetupRound::Id))
                    .col(timestamp(MatchyMeetupRound::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MatchyMeetupPair::Table)
                    .if_not_exists()
                    .col(pk_auto(MatchyMeetupPair::Id))
                    .col(integer(MatchyMeetupPair::RoundId))
                    .foreign_key(
                        ForeignKey::create()
                            .from(MatchyMeetupPair::Table, MatchyMeetupPair::RoundId)
                            .to(MatchyMeetupRound::Table, MatchyMeetupRound::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MatchyMeetupPairMember::Table)
                    .if_not_exists()
                    .col(integer(MatchyMeetupPairMember::PairId))
                    .col(big_integer(MatchyMeetupPairMember::DiscordUid))
                    .primary_key(
                        Index::create()
                            .col(MatchyMeetupPairMember::PairId)
                            .col(MatchyMeetupPairMember::DiscordUid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(MatchyMeetupPairMember::Table, MatchyMeetupPairMember::PairId)
                            .to(MatchyMeetupPair::Table, MatchyMeetupPair::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(MatchyMeetupPairMember::Table)
                    .cascade()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MatchyMeetupPair::Table)
                    .cascade()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MatchyMeetupRound::Table)
                    .cascade()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum MatchyMeetupRound {
    Table,
    Id,
    CreatedAt,
}

#[derive(DeriveIden)]
enum MatchyMeetupPair {
    Table,
    RoundId,
    Id,
}

#[derive(DeriveIden)]
enum MatchyMeetupPairMember {
    Table,
    PairId,
    DiscordUid,
}
